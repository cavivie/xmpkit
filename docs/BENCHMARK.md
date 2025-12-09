# XMPKit Performance Benchmarks

This document records the performance benchmark results of the XMPKit library.

## Test Environment

### Hardware Information
- **CPU**: Apple M1 (ARM64)
- **Physical Cores**: 8
- **Logical Cores**: 8
- **Memory**: 16 GB (17,179,869,184 bytes)
- **Operating System**: macOS 24.4.0 (Darwin Kernel Version 24.4.0)

### Software Environment
- **Rust Version**: 1.89.0 (29483883e 2025-08-04)
- **Cargo Version**: 1.89.0 (c24e10642 2025-06-23)
- **Testing Tool**: Criterion.rs 0.5

### Test Configuration
- All tests are compiled in release mode (`cargo bench`)
- Each test runs 100 samples
- Warm-up time: 3 seconds

---

## 1. XMP Parsing Performance (Parse Benchmarks)

Tests parsing performance of XMP packets with varying complexity.

| Test Scenario | Average Time | Description |
|--------------|--------------|-------------|
| **parse_simple** | 11.223 µs | Simple XMP packet (single property) |
| **parse_medium** | 16.990 µs | Medium complexity (multiple properties + arrays) |
| **parse_complex** | 30.188 µs | Complex XMP (multiple namespaces, arrays, structures) |
| **parse_large** | 38.807 µs | Large XMP packet (more properties, multilingual, large arrays, structures) |
| **parse_rdf_only** | 11.192 µs | Pure RDF (without xpacket wrapper) |
| **parse_from_str_trait** | 11.219 µs | Parsing using FromStr trait |

### Performance Analysis
- **Simple Parsing** (~11 µs): Parsing a single-property XMP packet is very fast
- **Complexity Impact**: Parsing time increases linearly with the number of properties, namespaces, and array items
- **xpacket Overhead**: Pure RDF parsing performs similarly to xpacket-wrapped parsing, indicating minimal wrapper overhead
- **FromStr Trait**: Trait-based parsing performs comparably to direct method calls

---

## 2. XMP Serialization Performance (Serialize Benchmarks)

Tests performance of serializing XMP metadata to XML/RDF format.

| Test Scenario | Average Time | Description |
|--------------|--------------|-------------|
| **serialize_simple** | 7.047 µs | Simple metadata serialization |
| **serialize_medium** | 9.765 µs | Medium complexity metadata serialization |
| **serialize_complex** | 12.729 µs | Complex metadata serialization |
| **serialize_packet_simple** | 6.165 µs | XMP Packet format serialization (simple) |
| **serialize_packet_complex** | 13.014 µs | XMP Packet format serialization (complex) |

### Performance Analysis
- **Serialization Speed**: Serialization is faster than parsing (~6-13 µs vs ~11-39 µs)
- **Complexity Impact**: Serialization time increases linearly with metadata complexity
- **Packet Format**: XMP Packet format (with `<?xpacket>` wrapper) has minimal overhead

---

## 3. Metadata Operations Performance (Metadata Operations Benchmarks)

Tests performance of various XMP metadata operations.

| Operation Type | Average Time | Description |
|----------------|--------------|-------------|
| **set_property** | 5.040 µs | Set property value |
| **get_property** | 241.35 ns | Get property value |
| **has_property** | 183.29 ns | Check if property exists |
| **delete_property** | 8.390 µs | Delete property |
| **append_array_item** | 4.982 µs | Append array item |
| **get_array_item** | 219.56 ns | Get array item |
| **get_array_size** | 162.56 ns | Get array size |
| **set_localized_text** | 5.238 µs | Set localized text |
| **get_localized_text** | 294.42 ns | Get localized text |

### Performance Analysis
- **Read Operations** (~180-300 ns): Extremely fast, suitable for frequent queries
- **Write Operations** (~5-8 µs): About 20-40x slower than reads, but still very fast
- **Array Operations**: Both reading and appending array items are efficient
- **Localized Text**: Multi-language localized text operations perform well

### Operation Performance Comparison
```
Read Operations (nanosecond level):
  has_property:         183 ns  ⚡ Fastest
  get_array_size:       163 ns
  get_array_item:       220 ns
  get_property:         241 ns
  get_localized_text:   294 ns

Write Operations (microsecond level):
  append_array_item:     5.0 µs
  set_property:          5.0 µs
  set_localized_text:    5.2 µs
  delete_property:       8.4 µs  ⚠️  Slowest
```

---

## 4. File I/O Performance (File I/O Benchmarks)

Tests performance of reading and writing XMP metadata from/to files.

| Test Scenario | Average Time | Description |
|--------------|--------------|-------------|
| **read_jpeg_from_bytes** | 61.369 µs | Read JPEG file from byte array |
| **read_jpeg_from_reader** | 58.925 µs | Read JPEG file from Reader |
| **write_jpeg_to_bytes** | 1.904 ms | Write XMP to JPEG file |
| **detect_format** | 29.749 ns | File format detection |

### Performance Analysis
- **File Reading** (~60 µs): Reading XMP metadata from JPEG files is very fast
- **Format Detection** (~30 ns): File format detection has almost no overhead
- **File Writing** (~1.9 ms): Writing operations are about 30x slower than reading because they require:
  - Reading the original file
  - Parsing file structure
  - Inserting/updating XMP segments
  - Writing the complete file

### I/O Performance Comparison
```
Format Detection:     30 ns    ⚡ Extremely fast
File Reading:         60 µs    ✅ Fast
File Writing:       1900 µs   ⚠️  Slower (but still acceptable)
```

---

## 5. Performance Summary

### Key Performance Metrics

| Operation Category | Fastest | Slowest | Average |
|-------------------|---------|---------|---------|
| **Parsing** | 11.2 µs | 38.8 µs | ~25 µs |
| **Serialization** | 6.2 µs | 13.0 µs | ~10 µs |
| **Metadata Reads** | 163 ns | 294 ns | ~220 ns |
| **Metadata Writes** | 5.0 µs | 8.4 µs | ~6 µs |
| **File Reading** | 59 µs | 61 µs | ~60 µs |
| **File Writing** | - | - | ~1900 µs |

### Performance Characteristics

1. **Excellent Parsing Performance**: 
   - Simple XMP packet parsing takes only ~11 µs
   - Complex XMP packet parsing takes only ~39 µs
   - Suitable for real-time processing scenarios

2. **Faster Serialization**: 
   - Serialization is about 2-3x faster than parsing
   - Simple metadata serialization takes only ~6 µs

3. **Efficient Metadata Operations**:
   - Read operations are at nanosecond level (~200 ns)
   - Write operations are at microsecond level (~5 µs)
   - Suitable for frequent metadata queries and modifications

4. **Good File I/O Performance**:
   - File reading is very fast (~60 µs)
   - File writing is slower but acceptable (~1.9 ms)
   - Format detection has almost no overhead (~30 ns)

### Use Cases

- ✅ **Real-time Processing**: Excellent parsing and serialization performance, suitable for real-time XMP metadata processing
- ✅ **Batch Processing**: Fast file reading, suitable for batch processing of large numbers of images
- ✅ **Frequent Queries**: Metadata read operations are at nanosecond level, suitable for frequent queries
- ✅ **In-memory Operations**: Excellent pure in-memory operation performance, suitable for constrained environments like Wasm

### Performance Optimization Recommendations

1. **Cache Parsed Results**: For XMP data that needs to be accessed multiple times, consider caching parsed results
2. **Batch Operations**: For processing multiple files, consider parallel processing
3. **Lazy Serialization**: Only serialize when needed, avoid unnecessary serialization operations
4. **Use Reader/Writer**: For large files, using Reader/Writer interfaces provides better memory control

---

## 6. Running Benchmarks

### Run All Tests
```bash
cargo bench
```

### Run Specific Tests
```bash
# Parsing performance tests
cargo bench --bench parse

# Serialization performance tests
cargo bench --bench serialize

# Metadata operations performance tests
cargo bench --bench metadata_ops

# File I/O performance tests
cargo bench --bench file_io
```

### View Detailed Reports
Benchmark results generate HTML reports located at:
```
target/criterion/<bench-name>/<function-name>/report/index.html
```

---

## 7. Changelog

- **2025-11-18**: Initial benchmark results
  - Test Environment: Apple M1, macOS 24.4.0
  - Rust Version: 1.89.0

---

## Appendix: Complete Test Results

### Parse Benchmarks
```
parse_simple            time:   [11.197 µs 11.223 µs 11.250 µs]
parse_medium            time:   [16.946 µs 16.990 µs 17.034 µs]
parse_complex           time:   [29.777 µs 30.188 µs 30.753 µs]
parse_large             time:   [38.690 µs 38.807 µs 38.927 µs]
parse_rdf_only          time:   [11.109 µs 11.192 µs 11.296 µs]
parse_from_str_trait    time:   [11.196 µs 11.219 µs 11.246 µs]
```

### Serialize Benchmarks
```
serialize_simple        time:   [6.5407 µs 7.0473 µs 7.6337 µs]
serialize_medium        time:   [9.2110 µs 9.7652 µs 10.481 µs]
serialize_complex        time:   [12.672 µs 12.729 µs 12.803 µs]
serialize_packet_simple  time:   [6.1436 µs 6.1646 µs 6.1884 µs]
serialize_packet_complex time:   [12.982 µs 13.014 µs 13.048 µs]
```

### Metadata Operations Benchmarks
```
set_property            time:   [4.9023 µs 5.0395 µs 5.2243 µs]
get_property            time:   [232.58 ns 241.35 ns 255.93 ns]
has_property            time:   [181.74 ns 183.29 ns 184.86 ns]
delete_property         time:   [8.2606 µs 8.3901 µs 8.6444 µs]
append_array_item       time:   [4.9410 µs 4.9824 µs 5.0288 µs]
get_array_item          time:   [217.24 ns 219.56 ns 221.93 ns]
get_array_size          time:   [155.38 ns 162.56 ns 176.27 ns]
set_localized_text      time:   [5.2087 µs 5.2381 µs 5.2731 µs]
get_localized_text      time:   [292.33 ns 294.42 ns 296.51 ns]
```

### File I/O Benchmarks
```
read_jpeg_from_bytes    time:   [60.739 µs 61.369 µs 62.554 µs]
read_jpeg_from_reader   time:   [58.816 µs 58.925 µs 59.052 µs]
write_jpeg_to_bytes     time:   [1.8393 ms 1.9040 ms 1.9854 ms]
detect_format           time:   [29.662 ns 29.749 ns 29.854 ns]
```
