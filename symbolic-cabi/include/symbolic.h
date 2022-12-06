/* C bindings to the symbolic library */

#ifndef SYMBOLIC_H_INCLUDED
#define SYMBOLIC_H_INCLUDED

/* Generated with cbindgen:0.24.3 */

/* Warning, this file is autogenerated. Do not modify this manually. */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Represents all possible error codes.
 */
enum SymbolicErrorCode {
  SYMBOLIC_ERROR_CODE_NO_ERROR = 0,
  SYMBOLIC_ERROR_CODE_PANIC = 1,
  SYMBOLIC_ERROR_CODE_UNKNOWN = 2,
  SYMBOLIC_ERROR_CODE_IO_ERROR = 101,
  SYMBOLIC_ERROR_CODE_UNKNOWN_ARCH_ERROR = 1001,
  SYMBOLIC_ERROR_CODE_UNKNOWN_LANGUAGE_ERROR = 1002,
  SYMBOLIC_ERROR_CODE_PARSE_DEBUG_ID_ERROR = 1003,
  SYMBOLIC_ERROR_CODE_UNKNOWN_OBJECT_KIND_ERROR = 2001,
  SYMBOLIC_ERROR_CODE_UNKNOWN_FILE_FORMAT_ERROR = 2002,
  SYMBOLIC_ERROR_CODE_OBJECT_ERROR_UNKNOWN = 2100,
  SYMBOLIC_ERROR_CODE_OBJECT_ERROR_UNSUPPORTED_OBJECT = 2101,
  SYMBOLIC_ERROR_CODE_OBJECT_ERROR_BAD_BREAKPAD_OBJECT = 2102,
  SYMBOLIC_ERROR_CODE_OBJECT_ERROR_BAD_ELF_OBJECT = 2103,
  SYMBOLIC_ERROR_CODE_OBJECT_ERROR_BAD_MACH_O_OBJECT = 2104,
  SYMBOLIC_ERROR_CODE_OBJECT_ERROR_BAD_PDB_OBJECT = 2105,
  SYMBOLIC_ERROR_CODE_OBJECT_ERROR_BAD_PE_OBJECT = 2106,
  SYMBOLIC_ERROR_CODE_OBJECT_ERROR_BAD_SOURCE_BUNDLE = 2107,
  SYMBOLIC_ERROR_CODE_OBJECT_ERROR_BAD_WASM_OBJECT = 2108,
  SYMBOLIC_ERROR_CODE_DWARF_ERROR_UNKNOWN = 2200,
  SYMBOLIC_ERROR_CODE_DWARF_ERROR_INVALID_UNIT_REF = 2201,
  SYMBOLIC_ERROR_CODE_DWARF_ERROR_INVALID_FILE_REF = 2202,
  SYMBOLIC_ERROR_CODE_DWARF_ERROR_UNEXPECTED_INLINE = 2203,
  SYMBOLIC_ERROR_CODE_DWARF_ERROR_INVERTED_FUNCTION_RANGE = 2204,
  SYMBOLIC_ERROR_CODE_DWARF_ERROR_CORRUPTED_DATA = 2205,
  SYMBOLIC_ERROR_CODE_CFI_ERROR_UNKNOWN = 3000,
  SYMBOLIC_ERROR_CODE_CFI_ERROR_MISSING_DEBUG_INFO = 3001,
  SYMBOLIC_ERROR_CODE_CFI_ERROR_UNSUPPORTED_DEBUG_FORMAT = 3002,
  SYMBOLIC_ERROR_CODE_CFI_ERROR_BAD_DEBUG_INFO = 3003,
  SYMBOLIC_ERROR_CODE_CFI_ERROR_UNSUPPORTED_ARCH = 3004,
  SYMBOLIC_ERROR_CODE_CFI_ERROR_WRITE_ERROR = 3005,
  SYMBOLIC_ERROR_CODE_CFI_ERROR_BAD_FILE_MAGIC = 3006,
  SYMBOLIC_ERROR_CODE_CFI_ERROR_INVALID_ADDRESS = 3007,
  SYMBOLIC_ERROR_CODE_PARSE_SOURCE_MAP_ERROR = 5001,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_UNKNOWN = 6000,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_BAD_FILE_MAGIC = 6001,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_BAD_FILE_HEADER = 6002,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_BAD_SEGMENT = 6003,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_BAD_CACHE_FILE = 6004,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_UNSUPPORTED_VERSION = 6005,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_BAD_DEBUG_FILE = 6006,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_MISSING_DEBUG_SECTION = 6007,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_MISSING_DEBUG_INFO = 6008,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_UNSUPPORTED_DEBUG_KIND = 6009,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_VALUE_TOO_LARGE = 6010,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_WRITE_FAILED = 6011,
  SYMBOLIC_ERROR_CODE_SYM_CACHE_ERROR_TOO_MANY_VALUES = 6012,
};
typedef uint32_t SymbolicErrorCode;

/**
 * A potential multi arch object.
 */
typedef struct SymbolicArchive SymbolicArchive;

/**
 * A `BCSymbolMap`.
 */
typedef struct SymbolicBcSymbolMap SymbolicBcSymbolMap;

/**
 * Contains stack frame information (CFI) for an image.
 */
typedef struct SymbolicCfiCache SymbolicCfiCache;

/**
 * A single arch object.
 */
typedef struct SymbolicObject SymbolicObject;

/**
 * Represents a proguard mapper.
 */
typedef struct SymbolicProguardMapper SymbolicProguardMapper;

/**
 * Represents an sourcemapcache.
 */
typedef struct SymbolicSourceMapCache SymbolicSourceMapCache;

/**
 * Represents a sourcemap view.
 */
typedef struct SymbolicSourceMapView SymbolicSourceMapView;

/**
 * Represents a source view.
 */
typedef struct SymbolicSourceView SymbolicSourceView;

/**
 * Represents a symbolic sym cache.
 */
typedef struct SymbolicSymCache SymbolicSymCache;

/**
 * A UUID mapping.
 */
typedef struct SymbolicUuidMapping SymbolicUuidMapping;

/**
 * A length-prefixed UTF-8 string.
 *
 * As opposed to C strings, this string is not null-terminated. If the string is owned, indicated
 * by the `owned` flag, the owner must call the `free` function on this string. The convention is:
 *
 *  - When obtained as instance through return values, always free the string.
 *  - When obtained as pointer through field access, never free the string.
 */
typedef struct SymbolicStr {
  /**
   * Pointer to the UTF-8 encoded string data.
   */
  char *data;
  /**
   * The length of the string pointed to by `data`.
   */
  uintptr_t len;
  /**
   * Indicates that the string is owned and must be freed.
   */
  bool owned;
} SymbolicStr;

/**
 * CABI wrapper around a UUID.
 */
typedef struct SymbolicUuid {
  /**
   * UUID bytes in network byte order (big endian).
   */
  uint8_t data[16];
} SymbolicUuid;

/**
 * Features this object contains.
 */
typedef struct SymbolicObjectFeatures {
  bool symtab;
  bool debug;
  bool unwind;
  bool sources;
} SymbolicObjectFeatures;

/**
 * Represents a Java Stack Frame.
 */
typedef struct SymbolicJavaStackFrame {
  struct SymbolicStr class_name;
  struct SymbolicStr method;
  struct SymbolicStr file;
  uintptr_t line;
} SymbolicJavaStackFrame;

/**
 * The result of remapping a Stack Frame.
 */
typedef struct SymbolicProguardRemapResult {
  struct SymbolicJavaStackFrame *frames;
  uintptr_t len;
} SymbolicProguardRemapResult;

/**
 * Represents a single token after lookup.
 */
typedef struct SymbolicTokenMatch {
  /**
   * The line number in the original source file.
   */
  uint32_t src_line;
  /**
   * The column number in the original source file.
   */
  uint32_t src_col;
  /**
   * The line number in the minifid source file.
   */
  uint32_t dst_line;
  /**
   * The column number in the minified source file.
   */
  uint32_t dst_col;
  /**
   * The source ID of the token.
   */
  uint32_t src_id;
  /**
   * The token name, if present.
   */
  struct SymbolicStr name;
  /**
   * The source.
   */
  struct SymbolicStr src;
  /**
   * The name of the function containing the token.
   */
  struct SymbolicStr function_name;
} SymbolicTokenMatch;

typedef struct SymbolicStrVec {
  struct SymbolicStr *strs;
  uintptr_t len;
} SymbolicStrVec;

/**
 * Represents a single token after lookup.
 */
typedef struct SymbolicSmTokenMatch {
  /**
   * The line number in the original source file.
   */
  uint32_t line;
  /**
   * The column number in the original source file.
   */
  uint32_t col;
  /**
   * The path to the original source.
   */
  struct SymbolicStr src;
  /**
   * The name of the source location as it is defined in the SourceMap.
   */
  struct SymbolicStr name;
  /**
   * The name of the function containing the token.
   */
  struct SymbolicStr function_name;
  struct SymbolicStrVec pre_context;
  struct SymbolicStr context_line;
  struct SymbolicStrVec post_context;
} SymbolicSmTokenMatch;

/**
 * Represents a single symbol after lookup.
 */
typedef struct SymbolicSourceLocation {
  uint64_t sym_addr;
  uint64_t instr_addr;
  uint32_t line;
  struct SymbolicStr lang;
  struct SymbolicStr symbol;
  struct SymbolicStr full_path;
} SymbolicSourceLocation;

/**
 * Represents a lookup result of one or more items.
 */
typedef struct SymbolicLookupResult {
  struct SymbolicSourceLocation *items;
  uintptr_t len;
} SymbolicLookupResult;

/**
 * Represents an instruction info.
 */
typedef struct SymbolicInstructionInfo {
  /**
   * The address of the instruction we want to use as a base.
   */
  uint64_t addr;
  /**
   * The architecture we are dealing with.
   */
  const struct SymbolicStr *arch;
  /**
   * This is true if the frame is the cause of the crash.
   */
  bool crashing_frame;
  /**
   * If a signal is know that triggers the crash, it can be stored here (0 if unknown)
   */
  uint32_t signal;
  /**
   * The optional value of the IP register (0 if unknown).
   */
  uint64_t ip_reg;
} SymbolicInstructionInfo;

/**
 * Extracts call frame information (CFI) from an Object.
 */
struct SymbolicCfiCache *symbolic_cficache_from_object(const struct SymbolicObject *object);

/**
 * Loads a CFI cache from the given path.
 */
struct SymbolicCfiCache *symbolic_cficache_open(const char *path);

/**
 * Returns the file format version of the CFI cache.
 */
uint32_t symbolic_cficache_get_version(const struct SymbolicCfiCache *cache);

/**
 * Returns a pointer to the raw buffer of the CFI cache.
 */
const uint8_t *symbolic_cficache_get_bytes(const struct SymbolicCfiCache *cache);

/**
 * Returns the size of the raw buffer of the CFI cache.
 */
uintptr_t symbolic_cficache_get_size(const struct SymbolicCfiCache *cache);

/**
 * Releases memory held by an unmanaged `SymbolicCfiCache` instance.
 */
void symbolic_cficache_free(struct SymbolicCfiCache *cache);

/**
 * Returns the latest CFI cache version.
 */
uint32_t symbolic_cficache_latest_version(void);

/**
 * Checks if an architecture is known.
 */
bool symbolic_arch_is_known(const struct SymbolicStr *arch);

/**
 * Normalizes an architecture name.
 */
struct SymbolicStr symbolic_normalize_arch(const struct SymbolicStr *arch);

/**
 * Returns the name of the instruction pointer if known.
 */
struct SymbolicStr symbolic_arch_ip_reg_name(const struct SymbolicStr *arch);

/**
 * Initializes the symbolic library.
 */
void symbolic_init(void);

/**
 * Returns the last error code.
 *
 * If there is no error, 0 is returned.
 */
SymbolicErrorCode symbolic_err_get_last_code(void);

/**
 * Returns the last error message.
 *
 * If there is no error an empty string is returned.  This allocates new memory
 * that needs to be freed with `symbolic_str_free`.
 */
struct SymbolicStr symbolic_err_get_last_message(void);

/**
 * Returns the panic information as string.
 */
struct SymbolicStr symbolic_err_get_backtrace(void);

/**
 * Clears the last error.
 */
void symbolic_err_clear(void);

/**
 * Creates a symbolic string from a raw C string.
 */
struct SymbolicStr symbolic_str_from_cstr(const char *string);

/**
 * Frees a symbolic str.
 *
 * If the string is marked as not owned then this function does not
 * do anything.
 */
void symbolic_str_free(struct SymbolicStr *string);

/**
 * Returns true if the uuid is nil.
 */
bool symbolic_uuid_is_nil(const struct SymbolicUuid *uuid);

/**
 * Formats the UUID into a string.
 *
 * The string is newly allocated and needs to be released with
 * `symbolic_cstr_free`.
 */
struct SymbolicStr symbolic_uuid_to_str(const struct SymbolicUuid *uuid);

/**
 * Loads an archive from a given path.
 */
struct SymbolicArchive *symbolic_archive_open(const char *path);

/**
 * Creates an archive from a byte buffer without taking ownership of the pointer.
 */
struct SymbolicArchive *symbolic_archive_from_bytes(const uint8_t *bytes,
                                                    uintptr_t len);

/**
 * Frees the given archive.
 */
void symbolic_archive_free(struct SymbolicArchive *archive);

/**
 * Returns the number of contained objects.
 */
uintptr_t symbolic_archive_object_count(const struct SymbolicArchive *archive);

/**
 * Returns the n-th object, or a null pointer if the object does not exist.
 */
struct SymbolicObject *symbolic_archive_get_object(const struct SymbolicArchive *archive,
                                                   uintptr_t index);

/**
 * Returns the architecture of the object.
 */
struct SymbolicStr symbolic_object_get_arch(const struct SymbolicObject *object);

/**
 * Returns the code identifier of the object.
 */
struct SymbolicStr symbolic_object_get_code_id(const struct SymbolicObject *object);

/**
 * Returns the debug identifier of the object.
 */
struct SymbolicStr symbolic_object_get_debug_id(const struct SymbolicObject *object);

/**
 * Returns the object kind (e.g. executable, debug file, library, ...).
 */
struct SymbolicStr symbolic_object_get_kind(const struct SymbolicObject *object);

/**
 * Returns the file format of the object file (e.g. MachO, ELF, ...).
 */
struct SymbolicStr symbolic_object_get_file_format(const struct SymbolicObject *object);

struct SymbolicObjectFeatures symbolic_object_get_features(const struct SymbolicObject *object);

/**
 * Frees an object returned from an archive.
 */
void symbolic_object_free(struct SymbolicObject *object);

/**
 * Loads a BCSymbolmap from a given path
 */
struct SymbolicBcSymbolMap *symbolic_bcsymbolmap_open(const char *path);

/**
 * Frees the given BcSymbolMap.
 */
void symbolic_bcsymbolmap_free(struct SymbolicBcSymbolMap *bcsymbolmap);

/**
 * Loads a UuidMapping by parsing the PList at `path`.
 */
struct SymbolicUuidMapping *symbolic_uuidmapping_from_plist(const struct SymbolicStr *debug_id,
                                                            const char *path);

/**
 * Frees the given UuidMapping.
 */
void symbolic_uuidmapping_free(struct SymbolicUuidMapping *mapping);

/**
 * Converts a Breakpad CodeModuleId to DebugId.
 */
struct SymbolicStr symbolic_id_from_breakpad(const struct SymbolicStr *breakpad_id);

/**
 * Normalizes a code identifier to default representation.
 */
struct SymbolicStr symbolic_normalize_code_id(const struct SymbolicStr *code_id);

/**
 * Normalizes a debug identifier to default representation.
 */
struct SymbolicStr symbolic_normalize_debug_id(const struct SymbolicStr *debug_id);

/**
 * Creates a proguard mapping view from a path.
 */
struct SymbolicProguardMapper *symbolic_proguardmapper_open(const char *path);

/**
 * Frees a proguard mapping view.
 */
void symbolic_proguardmapper_free(struct SymbolicProguardMapper *mapper);

/**
 * Remaps a Stack Frame.
 */
struct SymbolicProguardRemapResult symbolic_proguardmapper_remap_frame(const struct SymbolicProguardMapper *mapper,
                                                                       const struct SymbolicStr *class_,
                                                                       const struct SymbolicStr *method,
                                                                       uintptr_t line);

/**
 * Remaps a class name.
 */
struct SymbolicStr symbolic_proguardmapper_remap_class(const struct SymbolicProguardMapper *mapper,
                                                       const struct SymbolicStr *class_);

/**
 * Returns the UUID of a proguard mapping file.
 */
struct SymbolicUuid symbolic_proguardmapper_get_uuid(struct SymbolicProguardMapper *mapper);

/**
 * Returns true if the mapping file has line infos.
 */
bool symbolic_proguardmapper_has_line_info(const struct SymbolicProguardMapper *mapper);

/**
 * Frees a remap result.
 */
void symbolic_proguardmapper_result_free(struct SymbolicProguardRemapResult *result);

/**
 * Creates a source view from a given path.
 *
 * This shares the underlying memory and does not copy it if that is
 * possible.  Will ignore utf-8 decoding errors.
 */
struct SymbolicSourceView *symbolic_sourceview_from_bytes(const char *bytes,
                                                          uintptr_t len);

/**
 * Frees a source view.
 */
void symbolic_sourceview_free(struct SymbolicSourceView *view);

/**
 * Returns the underlying source (borrowed).
 */
struct SymbolicStr symbolic_sourceview_as_str(const struct SymbolicSourceView *view);

/**
 * Returns a specific line.
 */
struct SymbolicStr symbolic_sourceview_get_line(const struct SymbolicSourceView *view,
                                                uint32_t index);

/**
 * Returns the number of lines.
 */
uint32_t symbolic_sourceview_get_line_count(const struct SymbolicSourceView *source_map);

/**
 * Loads a sourcemap from a JSON byte slice.
 */
struct SymbolicSourceMapView *symbolic_sourcemapview_from_json_slice(const char *data,
                                                                     uintptr_t len);

/**
 * Frees a source map view.
 */
void symbolic_sourcemapview_free(struct SymbolicSourceMapView *source_map);

/**
 * Looks up a token.
 */
struct SymbolicTokenMatch *symbolic_sourcemapview_lookup_token(const struct SymbolicSourceMapView *source_map,
                                                               uint32_t line,
                                                               uint32_t col);

/**
 * Looks up a token and the original function name.
 *
 * This is similar to `lookup_token` but if a minified function name and
 * the sourceview to the minified source is available this function will
 * also resolve the original function name.  This is used to fully
 * resolve tracebacks.
 */
struct SymbolicTokenMatch *symbolic_sourcemapview_lookup_token_with_function_name(const struct SymbolicSourceMapView *source_map,
                                                                                  uint32_t line,
                                                                                  uint32_t col,
                                                                                  const struct SymbolicStr *minified_name,
                                                                                  const struct SymbolicSourceView *source_view);

/**
 * Return the sourceview for a given source.
 */
const struct SymbolicSourceView *symbolic_sourcemapview_get_sourceview(const struct SymbolicSourceMapView *source_map,
                                                                       uint32_t index);

/**
 * Return the source name for an index.
 */
struct SymbolicStr symbolic_sourcemapview_get_source_name(const struct SymbolicSourceMapView *source_map,
                                                          uint32_t index);

/**
 * Return the number of sources.
 */
uint32_t symbolic_sourcemapview_get_source_count(const struct SymbolicSourceMapView *source_map);

/**
 * Returns a specific token.
 */
struct SymbolicTokenMatch *symbolic_sourcemapview_get_token(const struct SymbolicSourceMapView *source_map,
                                                            uint32_t index);

/**
 * Returns the number of tokens.
 */
uint32_t symbolic_sourcemapview_get_tokens(const struct SymbolicSourceMapView *source_map);

/**
 * Free a token match.
 */
void symbolic_token_match_free(struct SymbolicTokenMatch *token_match);

/**
 * Creates an sourcemapcache from a given minified source and sourcemap contents.
 *
 * This shares the underlying memory and does not copy it if that is
 * possible.  Will ignore utf-8 decoding errors.
 */
struct SymbolicSourceMapCache *symbolic_sourcemapcache_from_bytes(const char *source_content,
                                                                  uintptr_t source_len,
                                                                  const char *sourcemap_content,
                                                                  uintptr_t sourcemap_len);

/**
 * Frees an SourceMapCache.
 */
void symbolic_sourcemapcache_free(struct SymbolicSourceMapCache *view);

/**
 * Looks up a token.
 */
struct SymbolicSmTokenMatch *symbolic_sourcemapcache_lookup_token(const struct SymbolicSourceMapCache *source_map,
                                                                  uint32_t line,
                                                                  uint32_t col,
                                                                  uint32_t context_lines);

/**
 * Free a token match.
 */
void symbolic_sourcemapcache_token_match_free(struct SymbolicSmTokenMatch *token_match);

/**
 * Creates a symcache from a given path.
 */
struct SymbolicSymCache *symbolic_symcache_open(const char *path);

/**
 * Creates a symcache from a byte buffer without taking ownership of the pointer.
 */
struct SymbolicSymCache *symbolic_symcache_from_bytes(const uint8_t *bytes,
                                                      uintptr_t len);

/**
 * Creates a symcache from a given object.
 */
struct SymbolicSymCache *symbolic_symcache_from_object(const struct SymbolicObject *object);

/**
 * Frees a symcache object.
 */
void symbolic_symcache_free(struct SymbolicSymCache *symcache);

/**
 * Returns the internal buffer of the symcache.
 *
 * The internal buffer is exactly `symbolic_symcache_get_size` bytes long.
 */
const uint8_t *symbolic_symcache_get_bytes(const struct SymbolicSymCache *symcache);

/**
 * Returns the size in bytes of the symcache.
 */
uintptr_t symbolic_symcache_get_size(const struct SymbolicSymCache *symcache);

/**
 * Returns the architecture of the symcache.
 */
struct SymbolicStr symbolic_symcache_get_arch(const struct SymbolicSymCache *symcache);

/**
 * Returns the architecture of the symcache.
 */
struct SymbolicStr symbolic_symcache_get_debug_id(const struct SymbolicSymCache *symcache);

/**
 * Returns the version of the cache file.
 */
uint32_t symbolic_symcache_get_version(const struct SymbolicSymCache *symcache);

/**
 * Looks up a single symbol.
 */
struct SymbolicLookupResult symbolic_symcache_lookup(const struct SymbolicSymCache *symcache,
                                                     uint64_t addr);

/**
 * Frees a lookup result.
 */
void symbolic_lookup_result_free(struct SymbolicLookupResult *lookup_result);

/**
 * Return the best instruction for an isntruction info.
 */
uint64_t symbolic_find_best_instruction(const struct SymbolicInstructionInfo *ii);

/**
 * Returns the latest symcache version.
 */
uint32_t symbolic_symcache_latest_version(void);

#endif /* SYMBOLIC_H_INCLUDED */
