//! Support for Portable Executables, an extension of COFF used on Windows.

use std::borrow::Cow;
use std::error::Error;
use std::fmt;

use gimli::RunTimeEndian;
use goblin::pe;
use thiserror::Error;

use symbolic_common::{Arch, AsSelf, CodeId, DebugId, Uuid};

use crate::base::*;
use crate::dwarf::*;
use crate::Parse;

pub use goblin::pe::exception::*;
pub use goblin::pe::section_table::SectionTable;

/// An error when dealing with [`PEObject`](struct.PEObject.html).
#[derive(Debug, Error)]
#[error("invalid PE file")]
pub struct PeError {
    #[source]
    source: Option<Box<dyn Error + Send + Sync + 'static>>,
}

impl PeError {
    /// Creates a new PE error from an arbitrary error payload.
    fn new<E>(source: E) -> Self
    where
        E: Into<Box<dyn Error + Send + Sync>>,
    {
        let source = Some(source.into());
        Self { source }
    }
}

/// Detects if the PE is a packer stub.
///
/// Such files usually only contain empty stubs in their `.pdata` and `.text` sections, and unwind
/// information cannot be retrieved reliably. Usually, the exception table is present, but unwind
/// info points into a missing section.
fn is_pe_stub(pe: &pe::PE<'_>) -> bool {
    let mut has_stub = false;
    let mut pdata_empty = false;

    for section in &pe.sections {
        let name = section.name().unwrap_or_default();
        pdata_empty = pdata_empty || name == ".pdata" && section.size_of_raw_data == 0;
        has_stub = has_stub || name.starts_with(".stub");
    }

    pdata_empty && has_stub
}

/// Portable Executable, an extension of COFF used on Windows.
///
/// This file format is used to carry program code. Debug information is usually moved to a separate
/// container, [`PdbObject`]. The PE file contains a reference to the PDB and vice versa to verify
/// that the files belong together.
///
/// In rare instances, PE files might contain debug information.
/// This is supported for DWARF debug information.
///
/// [`PdbObject`]: ../pdb/struct.PdbObject.html
pub struct PeObject<'data> {
    pe: pe::PE<'data>,
    data: &'data [u8],
    is_stub: bool,
}

impl<'data> PeObject<'data> {
    /// Tests whether the buffer could contain an PE object.
    pub fn test(data: &[u8]) -> bool {
        use scroll::{Pread, LE};
        matches!(
            data.get(0..2)
                .and_then(|data| data.pread_with::<u16>(0, LE).ok()),
            Some(pe::header::DOS_MAGIC)
        )
    }

    /// Tries to parse a PE object from the given slice.
    pub fn parse(data: &'data [u8]) -> Result<Self, PeError> {
        let pe = pe::PE::parse(data).map_err(PeError::new)?;
        let is_stub = is_pe_stub(&pe);
        Ok(PeObject { pe, data, is_stub })
    }

    /// The container file format, which is always `FileFormat::Pe`.
    pub fn file_format(&self) -> FileFormat {
        FileFormat::Pe
    }

    /// The code identifier of this object.
    ///
    /// The code identifier consists of the `time_date_stamp` field id the COFF header, followed by
    /// the `size_of_image` field in the optional header. If the optional PE header is not present,
    /// this identifier is `None`.
    pub fn code_id(&self) -> Option<CodeId> {
        let header = &self.pe.header;
        let optional_header = header.optional_header.as_ref()?;

        let timestamp = header.coff_header.time_date_stamp;
        let size_of_image = optional_header.windows_fields.size_of_image;
        let string = format!("{timestamp:08x}{size_of_image:x}");
        Some(CodeId::new(string))
    }

    /// The debug information identifier of this PE.
    ///
    /// Since debug information is usually stored in an external
    /// [`PdbObject`](crate::pdb::PdbObject), this identifier actually refers to the
    /// PDB. While strictly the filename of the PDB would also be necessary fully resolve
    /// it, in most instances the GUID and age contained in this identifier are sufficient.
    pub fn debug_id(&self) -> DebugId {
        self.pe
            .debug_data
            .as_ref()
            .and_then(|debug_data| debug_data.codeview_pdb70_debug_info.as_ref())
            .and_then(|debug_info| {
                // PE always stores the signature with little endian UUID fields.
                // Convert to network byte order (big endian) to match the
                // Breakpad processor's expectations.
                let mut data = debug_info.signature;
                data[0..4].reverse(); // uuid field 1
                data[4..6].reverse(); // uuid field 2
                data[6..8].reverse(); // uuid field 3

                let uuid = Uuid::from_slice(&data).ok()?;
                Some(DebugId::from_parts(uuid, debug_info.age))
            })
            .unwrap_or_default()
    }

    /// The name of the referenced PDB file.
    pub fn debug_file_name(&self) -> Option<Cow<'_, str>> {
        self.pe
            .debug_data
            .as_ref()
            .and_then(|debug_data| debug_data.codeview_pdb70_debug_info.as_ref())
            .map(|debug_info| {
                String::from_utf8_lossy(&debug_info.filename[..debug_info.filename.len() - 1])
            })
    }

    /// The CPU architecture of this object, as specified in the COFF header.
    pub fn arch(&self) -> Arch {
        let machine = self.pe.header.coff_header.machine;
        crate::pdb::arch_from_machine(machine.into())
    }

    /// The kind of this object, as specified in the PE header.
    pub fn kind(&self) -> ObjectKind {
        if self.pe.is_lib {
            ObjectKind::Library
        } else if self.is_stub {
            ObjectKind::Other
        } else {
            ObjectKind::Executable
        }
    }

    /// The address at which the image prefers to be loaded into memory.
    ///
    /// ELF files store all internal addresses as if it was loaded at that address. When the image
    /// is actually loaded, that spot might already be taken by other images and so it must be
    /// relocated to a new address. During load time, the loader rewrites all addresses in the
    /// program code to match the new load address so that there is no runtime overhead when
    /// executing the code.
    ///
    /// Addresses used in `symbols` or `debug_session` have already been rebased relative to that
    /// load address, so that the caller only has to deal with addresses relative to the actual
    /// start of the image.
    pub fn load_address(&self) -> u64 {
        self.pe.image_base as u64
    }

    /// Determines whether this object exposes a public symbol table.
    pub fn has_symbols(&self) -> bool {
        !self.pe.exports.is_empty()
    }

    /// Returns an iterator over symbols in the public symbol table.
    pub fn symbols(&self) -> PeSymbolIterator<'data, '_> {
        PeSymbolIterator {
            exports: self.pe.exports.iter(),
        }
    }

    /// Returns an ordered map of symbols in the symbol table.
    pub fn symbol_map(&self) -> SymbolMap<'data> {
        self.symbols().collect()
    }

    /// Determines whether this object contains debug information.
    ///
    /// Not usually the case, except for PE's generated by some alternative toolchains
    /// which contain DWARF debug info.
    pub fn has_debug_info(&self) -> bool {
        self.section(".debug_info").is_some()
    }

    /// Determines whether this object contains embedded source.
    pub fn has_sources(&self) -> bool {
        false
    }

    /// Determines whether this object is malformed and was only partially parsed
    pub fn is_malformed(&self) -> bool {
        false
    }

    /// Constructs a debugging session.
    ///
    /// A debugging session loads certain information from the object file and creates caches for
    /// efficient access to various records in the debug information. Since this can be quite a
    /// costly process, try to reuse the debugging session as long as possible.
    ///
    /// PE files usually don't have embedded debugging information,
    /// but some toolchains (e.g. MinGW) generate DWARF debug info.
    ///
    /// Constructing this session will also work if the object does not contain debugging
    /// information, in which case the session will be a no-op. This can be checked via
    /// [`has_debug_info`](struct.PeObject.html#method.has_debug_info).
    pub fn debug_session(&self) -> Result<DwarfDebugSession<'data>, DwarfError> {
        let symbols = self.symbol_map();
        DwarfDebugSession::parse(self, symbols, self.load_address() as i64, self.kind())
    }

    /// Determines whether this object contains stack unwinding information.
    pub fn has_unwind_info(&self) -> bool {
        !self.is_stub && self.exception_data().map_or(false, |e| !e.is_empty())
    }

    /// Returns the raw data of the PE file.
    pub fn data(&self) -> &'data [u8] {
        self.data
    }

    /// A list of the sections in this PE binary, used to resolve virtual addresses.
    pub fn sections(&self) -> &[SectionTable] {
        &self.pe.sections
    }

    /// Returns the `SectionTable` for the section with this name, if present.
    pub fn section(&self, name: &str) -> Option<SectionTable> {
        for s in &self.pe.sections {
            let sect_name = s.name();
            if sect_name.is_ok() && sect_name.unwrap() == name {
                return Some(s.clone());
            }
        }
        None
    }

    /// Returns exception data containing unwind information.
    pub fn exception_data(&self) -> Option<&ExceptionData<'_>> {
        if self.is_stub {
            None
        } else {
            self.pe.exception_data.as_ref()
        }
    }
}

impl fmt::Debug for PeObject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PeObject")
            .field("code_id", &self.code_id())
            .field("debug_id", &self.debug_id())
            .field("debug_file_name", &self.debug_file_name())
            .field("arch", &self.arch())
            .field("kind", &self.kind())
            .field("load_address", &format_args!("{:#x}", self.load_address()))
            .field("has_symbols", &self.has_symbols())
            .field("has_debug_info", &self.has_debug_info())
            .field("has_unwind_info", &self.has_unwind_info())
            .field("is_malformed", &self.is_malformed())
            .finish()
    }
}

impl<'slf, 'data: 'slf> AsSelf<'slf> for PeObject<'data> {
    type Ref = PeObject<'slf>;

    fn as_self(&'slf self) -> &Self::Ref {
        self
    }
}

impl<'data> Parse<'data> for PeObject<'data> {
    type Error = PeError;

    fn test(data: &[u8]) -> bool {
        Self::test(data)
    }

    fn parse(data: &'data [u8]) -> Result<Self, PeError> {
        Self::parse(data)
    }
}

impl<'data: 'object, 'object> ObjectLike<'data, 'object> for PeObject<'data> {
    type Error = DwarfError;
    type Session = DwarfDebugSession<'data>;
    type SymbolIterator = PeSymbolIterator<'data, 'object>;

    fn file_format(&self) -> FileFormat {
        self.file_format()
    }

    fn code_id(&self) -> Option<CodeId> {
        self.code_id()
    }

    fn debug_id(&self) -> DebugId {
        self.debug_id()
    }

    fn arch(&self) -> Arch {
        self.arch()
    }

    fn kind(&self) -> ObjectKind {
        self.kind()
    }

    fn load_address(&self) -> u64 {
        self.load_address()
    }

    fn has_symbols(&self) -> bool {
        self.has_symbols()
    }

    fn symbols(&'object self) -> Self::SymbolIterator {
        self.symbols()
    }

    fn symbol_map(&self) -> SymbolMap<'data> {
        self.symbol_map()
    }

    fn has_debug_info(&self) -> bool {
        self.has_debug_info()
    }

    fn debug_session(&self) -> Result<Self::Session, Self::Error> {
        self.debug_session()
    }

    fn has_unwind_info(&self) -> bool {
        self.has_unwind_info()
    }

    fn has_sources(&self) -> bool {
        self.has_sources()
    }

    fn is_malformed(&self) -> bool {
        self.is_malformed()
    }
}

/// An iterator over symbols in the PE file.
///
/// Returned by [`PeObject::symbols`](struct.PeObject.html#method.symbols).
pub struct PeSymbolIterator<'data, 'object> {
    exports: std::slice::Iter<'object, pe::export::Export<'data>>,
}

impl<'data, 'object> Iterator for PeSymbolIterator<'data, 'object> {
    type Item = Symbol<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        self.exports.next().map(|export| Symbol {
            name: export.name.map(Cow::Borrowed),
            address: export.rva as u64,
            size: export.size as u64,
        })
    }
}

impl<'data> Dwarf<'data> for PeObject<'data> {
    fn endianity(&self) -> RunTimeEndian {
        // According to https://reverseengineering.stackexchange.com/questions/17922/determining-endianness-of-pe-files-windows-on-arm,
        // the only known platform running PE's with big-endian code is the Xbox360. Probably not worth handling.
        RunTimeEndian::Little
    }

    fn raw_section(&self, name: &str) -> Option<DwarfSection<'data>> {
        // Name is given without leading "."
        let sect = self.section(&format!(".{}", name))?;
        let start = sect.pointer_to_raw_data as usize;
        let end = start + (sect.virtual_size as usize);
        let dwarf_data: &'data [u8] = self.data.get(start..end)?;
        let dwarf_sect = DwarfSection {
            // TODO: What about 64-bit PE+? Still 32 bit?
            address: u64::from(sect.virtual_address),
            data: Cow::from(dwarf_data),
            offset: u64::from(sect.pointer_to_raw_data),
            align: 4096, // TODO: Does goblin expose this? For now, assume 4K page size
        };
        Some(dwarf_sect)
    }
}
