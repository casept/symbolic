use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::io::{Seek, SeekFrom, Write};
use std::mem;
use std::slice;
use std::sync::Arc;

use symbolic_common::{Endianness, Error, ErrorKind, Result, ResultExt, Language};
use symbolic_debuginfo::{DwarfSection, Object};

use types::{CacheFileHeader, Seg, FuncRecord, LineRecord, FileRecord};
use utils::binsearch_by_key;
use cache::SYMCACHE_MAGIC;

use fallible_iterator::FallibleIterator;
use lru_cache::LruCache;
use fnv::FnvHashSet;
use gimli;
use gimli::{Abbreviations, AttributeValue, CompilationUnitHeader, DebugAbbrev, DebugAbbrevOffset,
            DebugInfo, DebugInfoOffset, DebugLine, DebugLineOffset, DebugRanges, DebugStr,
            DebuggingInformationEntry, DwLang, EndianBuf, StateMachine,
            IncompleteLineNumberProgram, Range, UnitOffset};

type Buf<'input> = EndianBuf<'input, Endianness>;
type Die<'abbrev, 'unit, 'input> = DebuggingInformationEntry<'abbrev, 'unit, Buf<'input>>;

fn err(msg: &'static str) -> Error {
    Error::from(ErrorKind::BadDwarfData(msg))
}

/// Given a writer and object, dumps the object into the writer.
///
/// In case a symcache is to be constructed from memory the `SymCache::from_object`
/// method can be used instead.
///
/// This requires the writer to be seekable.
pub fn to_writer<W: Write + Seek>(mut w: W, obj: &Object) -> Result<()> {
    w.write_all(CacheFileHeader::default().as_bytes())?;
    let header = {
        let mut writer = SymCacheWriter::new(&mut w);
        writer.write_object(obj)?;
        writer.header
    };
    w.seek(SeekFrom::Start(0))?;
    w.write_all(header.as_bytes())?;
    Ok(())
}

/// Converts an object into a vector of symcache data.
pub fn to_vec(obj: &Object) -> Result<Vec<u8>> {
    let mut buf = Vec::<u8>::new();
    buf.write_all(CacheFileHeader::default().as_bytes())?;
    let header = {
        let mut writer = SymCacheWriter::new(&mut buf);
        writer.write_object(obj)?;
        writer.header
    };
    let header_bytes = header.as_bytes();
    (&mut buf[..header_bytes.len()]).copy_from_slice(header_bytes);
    Ok(buf)
}

struct Function<'a> {
    pub depth: u16,
    pub len: u32,
    pub name: &'a [u8],
    pub inlines: Vec<Function<'a>>,
    pub lines: Vec<Line<'a>>,
    pub comp_dir: &'a [u8],
    pub lang: Language,
}

struct Line<'a> {
    pub addr: u64,
    pub original_file_id: u64,
    pub filename: &'a [u8],
    pub base_dir: &'a [u8],
    pub line: u32,
}

impl<'a> fmt::Debug for Line<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Line")
            .field("addr", &self.addr)
            .field("base_dir", &String::from_utf8_lossy(self.base_dir))
            .field("original_file_id", &self.original_file_id)
            .field("filename", &String::from_utf8_lossy(self.filename))
            .field("line", &self.line)
            .finish()
    }
}

impl<'a> fmt::Debug for Function<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Function")
            .field("name", &String::from_utf8_lossy(self.name))
            .field("depth", &self.depth)
            .field("inlines", &self.inlines)
            .field("comp_dir", &self.comp_dir)
            .field("lang", &self.lang)
            .field("lines", &self.lines)
            .finish()
    }
}

impl<'a> Function<'a> {
    pub fn append_line_if_changed(&mut self, line: Line<'a>) {
        if let Some(last_line) = self.lines.last() {
            if last_line.original_file_id == line.original_file_id &&
               last_line.line == line.line
            {
                return;
            }
        }
        self.lines.push(line);
    }

    pub fn dedup_inlines(&mut self) {
        let mut addrs = FnvHashSet::default();
        self.dedup_inlines_walk(&mut addrs);
    }

    fn dedup_inlines_walk(&mut self, addrs: &mut FnvHashSet<u64>) {
        for func in self.inlines.iter_mut() {
            func.dedup_inlines_walk(addrs);
        }
        self.lines.retain(|item| !addrs.contains(&item.addr));
        for line in &self.lines {
            addrs.insert(line.addr);
        }
    }

    pub fn get_addr(&self) -> u64 {
        if let Some(line) = self.lines.get(0) {
            line.addr
        } else if let Some(func) = self.inlines.get(0) {
            func.get_addr()
        } else {
            0
        }
    }
}

struct SymCacheWriter<W: Write> {
    writer: RefCell<(u64, W)>,
    header: CacheFileHeader,
    symbol_map: HashMap<Vec<u8>, u32>,
    symbols: Vec<Seg<u8>>,
    files: HashMap<Vec<u8>, Seg<u8>>,
    file_record_map: HashMap<FileRecord, u16>,
    file_records: Vec<FileRecord>,
    func_records: Vec<FuncRecord>,
    line_records: Vec<Seg<LineRecord>>,
}

#[derive(Debug)]
struct DwarfInfo<'input> {
    pub units: Vec<CompilationUnitHeader<Buf<'input>>>,
    pub debug_abbrev: DebugAbbrev<Buf<'input>>,
    pub debug_ranges: DebugRanges<Buf<'input>>,
    pub debug_line: DebugLine<Buf<'input>>,
    pub debug_str: DebugStr<Buf<'input>>,
    abbrev_cache: RefCell<LruCache<DebugAbbrevOffset<usize>, Arc<Abbreviations>>>,
}

impl<'input> DwarfInfo<'input> {
    pub fn from_object(obj: &'input Object) -> Result<DwarfInfo<'input>> {
        macro_rules! section {
            ($sect:ident, $mandatory:expr) => {{
                let sect = match obj.get_dwarf_section(DwarfSection::$sect) {
                    Some(sect) => sect.as_bytes(),
                    None => {
                        if $mandatory {
                            return Err(ErrorKind::MissingSection(
                                DwarfSection::$sect.get_elf_section()).into());
                        }
                        &[]
                    }
                };
                $sect::new(sect, obj.endianess())
            }}
        }

        Ok(DwarfInfo {
            units: section!(DebugInfo, true).units().collect()?,
            debug_abbrev: section!(DebugAbbrev, true),
            debug_line: section!(DebugLine, true),
            debug_ranges: section!(DebugRanges, false),
            debug_str: section!(DebugStr, false),
            abbrev_cache: RefCell::new(LruCache::new(30)),
        })
    }

    pub fn get_unit_header(&self, index: usize)
        -> Result<&CompilationUnitHeader<Buf<'input>>>
    {
        self.units.get(index).ok_or_else(|| err("non existing unit"))
    }

    pub fn get_abbrev(&self, header: &CompilationUnitHeader<Buf<'input>>)
        -> Result<Arc<Abbreviations>>
    {
        let offset = header.debug_abbrev_offset();
        let mut cache = self.abbrev_cache.borrow_mut();
        if let Some(abbrev) = cache.get_mut(&offset) {
            return Ok(abbrev.clone());
        }
        let abbrev = header
            .abbreviations(&self.debug_abbrev)
            .chain_err(|| {
                err("compilation unit refers to non-existing abbreviations")
            })?;
        cache.insert(offset, Arc::new(abbrev));
        Ok(cache.get_mut(&offset).unwrap().clone())
    }

    fn find_unit_offset(&self, offset: DebugInfoOffset<usize>)
        -> Result<(usize, UnitOffset<usize>)>
    {
        match binsearch_by_key(&self.units, offset.0, |_, x| x.offset().0) {
            Some((index, header)) => {
                if let Some(unit_offset) = offset.to_unit_offset(header) {
                    return Ok((index, unit_offset));
                }
            }
            None => {}
        }
        Err(err("couln't find unit for ref address"))
    }
}

impl<W: Write> SymCacheWriter<W> {
    pub fn new(writer: W) -> SymCacheWriter<W> {
        SymCacheWriter {
            writer: RefCell::new((0, writer)),
            header: Default::default(),
            symbol_map: HashMap::new(),
            symbols: vec![],
            files: HashMap::new(),
            file_record_map: HashMap::new(),
            file_records: vec![],
            func_records: vec![],
            line_records: vec![],
        }
    }

    #[inline(always)]
    fn write_bytes(&self, bytes: &[u8]) -> Result<Seg<u8>> {
        let (ref mut pos, ref mut writer) = *self.writer.borrow_mut();
        let offset = *pos;
        *pos += bytes.len() as u64;
        writer.write_all(bytes)?;
        Ok(Seg::new(offset as u32, bytes.len() as u32))
    }

    #[inline(always)]
    fn write_item<T>(&self, x: &T) -> Result<Seg<u8>> {
        unsafe {
            let bytes: *const u8 = mem::transmute(x);
            let size = mem::size_of_val(x);
            self.write_bytes(slice::from_raw_parts(bytes, size))
        }
    }

    #[inline]
    fn write_seg<T>(&self, x: &[T]) -> Result<Seg<T>> {
        let mut first_seg = None;
        for item in x {
            let seg = self.write_item(item)?;
            if first_seg.is_none() {
                first_seg = Some(seg);
            }
        }
        Ok(Seg::new(first_seg.map(|x| x.offset).unwrap_or(0), x.len() as u32))
    }

    fn write_symbol_if_missing(&mut self, sym: &[u8]) -> Result<u32> {
        if let Some(&index) = self.symbol_map.get(sym) {
            return Ok(index);
        }
        let idx = self.symbols.len() as u32;
        let seg = self.write_bytes(sym)?;
        self.symbols.push(seg);
        self.symbol_map.insert(sym.to_owned(), idx);
        Ok(idx)
    }

    #[inline]
    fn write_file_if_missing(&mut self, filename: &[u8]) -> Result<Seg<u8>> {
        if let Some(item) = self.files.get(filename) {
            return Ok(*item);
        }
        let seg = self.write_bytes(filename)?;
        self.files.insert(filename.to_owned(), seg);
        Ok(seg)
    }

    fn write_file_record_if_missing(&mut self, record: FileRecord)
        -> Result<u16>
    {
        if let Some(idx) = self.file_record_map.get(&record) {
            return Ok(*idx);
        }
        let idx = self.file_records.len() as u16;
        self.file_record_map.insert(record, idx);
        self.file_records.push(record);
        Ok(idx)
    }

    pub fn write_object(&mut self, obj: &Object) -> Result<()> {
        let info = DwarfInfo::from_object(obj)
            .chain_err(|| err("could not extract debug info from object file"))?;

        let mut range_buf = Vec::new();
        for index in 0..info.units.len() {
            // attempt to parse a single unit from the given header.
            let unit_opt = Unit::parse(&info, index)
                .chain_err(|| err("encountered invalid compilation unit"))?;

            // skip units we don't care about
            let unit = match unit_opt {
                Some(unit) => unit,
                None => continue,
            };

            // dedup instructions from inline functions
            unit.for_each_function(&info, &mut range_buf, |mut func| {
                func.dedup_inlines();
                self.write_function(&func, !0)
            })?;
        }

        self.header.magic = SYMCACHE_MAGIC;
        self.header.version = 1;
        self.header.arch = obj.arch() as u32;
        if let Some(uuid) = obj.uuid() {
            self.header.uuid = uuid;
        }

        self.header.symbols = self.write_seg(&self.symbols)?;
        self.header.files = self.write_seg(&self.file_records)?;
        self.header.function_records = self.write_seg(&self.func_records)?;
        self.header.line_records = self.write_seg(&self.line_records)?;

        Ok(())
    }

    fn write_function<'a>(&mut self, func: &Function<'a>, parent_id: u32)
        -> Result<()>
    {
        let mut to_write = VecDeque::new();
        to_write.push_front((func, parent_id));
        let mut local_cache = HashMap::new();

        while let Some((func, parent_id)) = to_write.pop_front() {
            let func_addr = func.get_addr();
            let func_id = self.func_records.len() as u32;

            let mut func_record = FuncRecord {
                addr_low: (func_addr & 0xffffffff) as u32,
                addr_high: ((func_addr << 32) & 0xffff) as u16,
                // XXX: overflow needs to write a second func record
                len: func.len as u16,
                symbol_id: self.write_symbol_if_missing(func.name)?,
                parent_id: parent_id,
                line_record_id: !0,
                comp_dir: self.write_file_if_missing(func.comp_dir)?,
                lang: func.lang as u32,
            };

            let mut line_records = vec![];
            let mut last_addr = func_record.addr_start();
            for line in &func.lines {
                let file_id = if let Some(&x) = local_cache.get(&line.original_file_id) {
                    x
                } else {
                    let file_record = FileRecord {
                        filename: self.write_file_if_missing(line.filename)?,
                        base_dir: self.write_file_if_missing(line.base_dir)?,
                    };
                    let file_id = self.write_file_record_if_missing(file_record)?;
                    local_cache.insert(line.original_file_id, file_id);
                    file_id
                };

                // XXX: handle overflows as multiple records
                let line_record = LineRecord {
                    addr_off: (line.addr - last_addr) as u16,
                    file_id: file_id,
                    line: line.line as u16,
                };

                last_addr += line_record.addr_off as u64;
                line_records.push(line_record);
            }

            if !line_records.is_empty() {
                let seg = self.write_seg(&line_records)?;
                let line_record_id = self.line_records.len() as u32;
                self.line_records.push(seg);
                func_record.line_record_id = line_record_id;
            }

            self.func_records.push(func_record);
            for inline_func in &func.inlines {
                to_write.push_back((inline_func, func_id));
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Unit<'input> {
    index: usize,
    base_address: u64,
    comp_dir: Option<Buf<'input>>,
    comp_name: Option<Buf<'input>>,
    language: Option<DwLang>,
    line_offset: DebugLineOffset,
}

impl<'input> Unit<'input> {
    fn parse(info: &DwarfInfo<'input>, index: usize) -> Result<Option<Unit<'input>>> {
        let header = info.get_unit_header(index)?;

        // Access the compilation unit, which must be the top level DIE
        let abbrev = info.get_abbrev(header)?;
        let mut entries = header.entries(&*abbrev);
        let (_, entry) = entries
            .next_dfs()
            .chain_err(|| err("compilation unit is broken"))?
            .ok_or_else(|| err("unit without compilation unit"))?;

        if entry.tag() != gimli::DW_TAG_compile_unit {
            return Err(err("missing compilation unit"));
        }

        let base_address = match entry.attr_value(gimli::DW_AT_low_pc) {
            Ok(Some(AttributeValue::Addr(addr))) => addr,
            Err(e) => {
                return Err(e).chain_err(|| err("invalid low_pc attribute"));
            }
            _ => {
                match entry.attr_value(gimli::DW_AT_entry_pc) {
                    Ok(Some(AttributeValue::Addr(addr))) => addr,
                    Err(e) => {
                        return Err(e).chain_err(|| err("invalid entry_pc attribute"));
                    }
                    _ => 0,
                }
            }
        };

        let comp_dir = entry
            .attr(gimli::DW_AT_comp_dir)
            .chain_err(|| err("invalid compilation unit directory"))?
            .and_then(|attr| attr.string_value(&info.debug_str));

        let comp_name = entry
            .attr(gimli::DW_AT_name)
            .chain_err(|| err("invalid compilation unit name"))?
            .and_then(|attr| attr.string_value(&info.debug_str));

        let language = entry
            .attr(gimli::DW_AT_language)
            .chain_err(|| err("invalid language"))?
            .and_then(|attr| match attr.value() {
                AttributeValue::Language(lang) => Some(lang),
                _ => None,
            });

        let line_offset = match entry.attr_value(gimli::DW_AT_stmt_list) {
            Ok(Some(AttributeValue::DebugLineRef(offset))) => offset,
            Err(e) => {
                return Err(e).chain_err(|| "invalid compilation unit statement list");
            }
            _ => {
                return Ok(None);
            }
        };

        Ok(Some(Unit {
            index,
            base_address,
            comp_dir,
            comp_name,
            language,
            line_offset,
        }))
    }

    fn for_each_function<T, F>(&self, info: &DwarfInfo<'input>,
                               range_buf: &mut Vec<Range>, mut f: F)
        -> Result<()>
    where F: FnMut(Function<'input>) -> Result<T>
    {
        let mut depth = 0;
        let header = info.get_unit_header(self.index)?;
        let abbrev = info.get_abbrev(header)?;
        let mut entries = header.entries(&*abbrev);
        let mut last_func: Option<Function> = None;

        let line_program = DwarfLineProgram::parse(
            info,
            self.line_offset,
            header.address_size(),
            self.comp_dir,
            self.comp_name,
        )?;

        while let Some((movement, entry)) = entries
            .next_dfs()
            .chain_err(|| err("tree below compilation unit yielded invalid entry"))?
        {
            depth += movement;

            // skip anything that is not a function
            let inline = match entry.tag() {
                gimli::DW_TAG_subprogram => false,
                gimli::DW_TAG_inlined_subroutine => true,
                _ => continue,
            };

            let ranges = self.parse_ranges(info, entry, range_buf)
                .chain_err(|| err("subroutine has invalid ranges"))?;
            if ranges.is_empty() {
                continue;
            }

            let mut func = Function {
                depth: depth as u16,
                len: (ranges[ranges.len() - 1].end - ranges[0].begin) as u32,
                name: self.resolve_function_name(info, entry)?.unwrap_or(b""),
                inlines: vec![],
                lines: vec![],
                comp_dir: self.comp_dir.map(|x| x.buf()).unwrap_or(b""),
                lang: self.language
                    .and_then(|lang| Language::from_dwarf_lang(lang))
                    .unwrap_or(Language::Unknown)
            };

            for range in ranges {
                let rows = line_program.get_rows(range);
                for row in rows {
                    let (base_dir, filename) = line_program.get_filename(row.file_index)?;

                    let new_line = Line {
                        addr: row.address,
                        original_file_id: row.file_index as u64,
                        filename: filename,
                        base_dir: base_dir,
                        line: row.line.unwrap_or(0) as u32,
                    };

                    func.append_line_if_changed(new_line);
                }
            }

            if inline {
                let mut node = last_func.as_mut()
                    .ok_or_else(|| err("no root function"))?;
                while {&node}.inlines.last().map_or(false, |n| (n.depth as isize) < depth) {
                    node = {node}.inlines.last_mut().unwrap();
                }
                node.inlines.push(func);
            } else {
                if let Some(func) = last_func {
                    f(func)?;
                }
                last_func = Some(func);
            }
        }

        if let Some(func) = last_func {
            f(func)?;
        }

        Ok(())
    }

    fn parse_ranges<'a>(&self, info: &DwarfInfo<'input>, entry: &Die,
                        buf: &'a mut Vec<Range>) -> Result<&'a [Range]> {
        let mut low_pc = None;
        let mut high_pc = None;
        let mut high_pc_rel = None;

        buf.clear();

        macro_rules! set_pc {
            ($var:ident, $val:expr) => {{
                $var = Some($val);
                if low_pc.is_some() && (high_pc.is_some() || high_pc_rel.is_some()) {
                    break;
                }
            }}
        }

        let mut fiter = entry.attrs();
        while let Some(attr) = fiter.next()? {
            match attr.name() {
                gimli::DW_AT_ranges => {
                    match attr.value() {
                        AttributeValue::DebugRangesRef(offset) => {
                            let header = info.get_unit_header(self.index)?;
                            let mut fiter = info.debug_ranges
                                .ranges(offset, header.address_size(), self.base_address)
                                .chain_err(|| err("range offsets are not valid"))?;

                            while let Some(item) = fiter.next()? {
                                buf.push(item);
                            }
                            return Ok(&buf[..]);
                        }
                        // XXX: error?
                        _ => continue
                    }
                }
                gimli::DW_AT_low_pc => {
                    match attr.value() {
                        AttributeValue::Addr(addr) => set_pc!(low_pc, addr),
                        _ => return Ok(&[])
                    }
                }
                gimli::DW_AT_high_pc => {
                    match attr.value() {
                        AttributeValue::Addr(addr) => set_pc!(high_pc, addr),
                        AttributeValue::Udata(size) => set_pc!(high_pc_rel, size),
                        _ => return Ok(&[])
                    }
                }
                _ => continue
            }
        }

        // to go by the logic in dwarf2read a low_pc of 0 can indicate an
        // eliminated duplicate when the GNU linker is used.
        // TODO: *technically* there could be a relocatable section placed at VA 0
        let low_pc = match low_pc {
            Some(low_pc) if low_pc != 0 => low_pc,
            _ => return Ok(&[])
        };

        let high_pc = match (high_pc, high_pc_rel) {
            (Some(high_pc), _) => high_pc,
            (_, Some(high_pc_rel)) => low_pc.wrapping_add(high_pc_rel),
            _ => return Ok(&[])
        };

        if low_pc == high_pc {
            // most likely low_pc == high_pc means the DIE should be ignored.
            // https://sourceware.org/ml/gdb-patches/2011-03/msg00739.html
            return Ok(&[]);
        }

        if low_pc > high_pc {
            // XXX: consider swallowing errors?
            return Err(err("invalid due to inverted range"));
        }

        buf.push(Range {
            begin: low_pc,
            end: high_pc,
        });
        Ok(&buf[..])
    }

    /// Resolves an entry and if found invokes a function to transform it.
    ///
    /// As this might resolve into cached information the data borrowed from
    /// abbrev can only be temporarily accessed in the callback.
    fn resolve_reference<'info, T, F>(
        &self,
        info: &'info DwarfInfo<'input>,
        attr_value: AttributeValue<Buf<'input>>,
        f: F,
    ) -> Result<Option<T>>
        where for<'abbrev> F: FnOnce(&Die<'abbrev, 'info, 'input>) -> Result<Option<T>>
    {
        let (index, offset) = match attr_value {
            AttributeValue::UnitRef(offset) => {
                (self.index, offset)
            }
            AttributeValue::DebugInfoRef(offset) => {
                let (index, unit_offset) = info.find_unit_offset(offset)?;
                (index, unit_offset)
            }
            // TODO: there is probably more that can come back here
            _ => { return Ok(None); }
        };

        let header = info.get_unit_header(index)?;
        let abbrev = info.get_abbrev(header)?;
        let mut entries = header.entries_at_offset(&*abbrev, offset)?;
        entries.next_entry()?;
        if let Some(entry) = entries.current() {
            f(entry)
        } else {
            Ok(None)
        }
    }

    /// Resolves the function name of a debug entry.
    fn resolve_function_name<'abbrev, 'unit>(
        &self,
        info: &DwarfInfo<'input>,
        entry: &Die<'abbrev, 'unit, 'input>,
    ) -> Result<Option<&'input [u8]>> {
        let mut fiter = entry.attrs();
        let mut fallback_name = None;
        let mut reference_target = None;

        while let Some(attr) = fiter.next()? {
            match attr.name() {
                // prioritize these.  If we get them, take them.
                gimli::DW_AT_linkage_name |
                gimli::DW_AT_MIPS_linkage_name => {
                    return Ok(attr.string_value(&info.debug_str).map(|x| x.buf()));
                }
                gimli::DW_AT_name => {
                    fallback_name = Some(attr);
                }
                gimli::DW_AT_abstract_origin |
                gimli::DW_AT_specification => {
                    reference_target = Some(attr);
                }
                _ => {}
            }
        }

        if let Some(attr) = fallback_name {
            return Ok(attr.string_value(&info.debug_str).map(|x| x.buf()));
        }

        if let Some(attr) = reference_target {
            if let Some(name) = self.resolve_reference(info, attr.value(), |ref_entry| {
                self.resolve_function_name(info, ref_entry)
                    .chain_err(|| err("reference does not resolve to a name"))
            })? {
                return Ok(Some(name));
            }
        }

        Ok(None)
    }
}

#[derive(Debug)]
struct DwarfLineProgram<'input> {
    sequences: Vec<DwarfSeq>,
    program_rows: StateMachine<Buf<'input>, IncompleteLineNumberProgram<Buf<'input>>>,
}

#[derive(Debug)]
struct DwarfSeq {
    low_address: u64,
    high_address: u64,
    rows: Vec<DwarfRow>,
}

#[derive(Debug, PartialEq, Eq)]
struct DwarfRow {
    address: u64,
    file_index: u64,
    line: Option<u64>,
}

impl<'input> DwarfLineProgram<'input> {
    fn parse<'info>(
        info: &'info DwarfInfo<'input>,
        line_offset: DebugLineOffset,
        address_size: u8,
        comp_dir: Option<Buf<'input>>,
        comp_name: Option<Buf<'input>>,
    ) -> Result<Self> {
        let program = info.debug_line
            .program(line_offset, address_size, comp_dir, comp_name)?;

        let mut sequences = vec![];
        let mut sequence_rows: Vec<DwarfRow> = vec![];
        let mut prev_address = 0;
        let mut program_rows = program.rows();

        while let Ok(Some((_, &program_row))) = program_rows.next_row() {
            let address = program_row.address();
            if program_row.end_sequence() {
                if !sequence_rows.is_empty() {
                    let low_address = sequence_rows[0].address;
                    let high_address = if address < prev_address {
                        prev_address + 1
                    } else {
                        address
                    };
                    let mut rows = vec![];
                    mem::swap(&mut rows, &mut sequence_rows);
                    sequences.push(DwarfSeq {
                        low_address,
                        high_address,
                        rows,
                    });
                }
                prev_address = 0;
            } else if address < prev_address {
                // The standard says:
                // "Within a sequence, addresses and operation pointers may only increase."
                // So this row is invalid, we can ignore it.
                //
                // If we wanted to handle this, we could start a new sequence
                // here, but let's wait until that is needed.
            } else {
                let file_index = program_row.file_index();
                let line = program_row.line();
                let mut duplicate = false;
                if let Some(last_row) = sequence_rows.last_mut() {
                    if last_row.address == address {
                        last_row.file_index = file_index;
                        last_row.line = line;
                        duplicate = true;
                    }
                }
                if !duplicate {
                    sequence_rows.push(DwarfRow {
                        address,
                        file_index,
                        line,
                    });
                }
                prev_address = address;
            }
        }
        if !sequence_rows.is_empty() {
            // A sequence without an end_sequence row.
            // Let's assume the last row covered 1 byte.
            let low_address = sequence_rows[0].address;
            let high_address = prev_address + 1;
            sequences.push(DwarfSeq {
                low_address,
                high_address,
                rows: sequence_rows,
            });
        }

        // XXX: assert everything is sorted

        Ok(DwarfLineProgram {
            sequences: sequences,
            program_rows: program_rows,
        })
    }

    pub fn get_filename(&self, idx: u64) -> Result<(&'input [u8], &'input [u8])> {
        let header = self.program_rows.header();
        let file = header
            .file(idx)
            .ok_or_else(|| ErrorKind::BadDwarfData("invalid file reference"))?;
        Ok((
            file.directory(header).map(|x| x.buf()).unwrap_or(b""),
            file.path_name().buf()
        ))
    }

    pub fn get_rows(&self, rng: &Range) -> &[DwarfRow] {
        for seq in &self.sequences {
            if seq.high_address < rng.begin || seq.low_address > rng.end {
                continue;
            }

            let start = match binsearch_by_key(&seq.rows, rng.begin, |_, x| x.address) {
                Some((idx, _)) => idx,
                None => { continue; }
            };
            return match binsearch_by_key(&seq.rows[start..], rng.end, |_, x| x.address) {
                Some((idx, _)) => &seq.rows[start..start + idx],
                None => &seq.rows[start..],
            };
        }
        &[]
    }
}