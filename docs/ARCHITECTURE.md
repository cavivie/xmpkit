# XMPKit Architecture

This document describes the architecture and design of XMPKit.

## Overview

XMPKit is organized into several core modules:

- **Core**: XMP metadata parsing, manipulation, and serialization
- **Files**: File format handlers for reading/writing XMP from files
- **Types**: Common types and data structures
- **Utils**: Utility functions (e.g., date/time handling)

## Core Module

The core module (`src/core/`) provides the fundamental XMP functionality:

### Metadata (`metadata.rs`)

- `XmpMeta`: Main structure for working with XMP metadata
- Provides APIs for reading, writing, and manipulating properties
- Supports arrays, structures, localized text, and date/time properties

### Parser (`parser.rs`)

- `XmpParser`: Parses XMP Packets from RDF/XML format
- Handles XML entity decoding
- Supports dynamic namespace registration

### Serializer (`serializer.rs`)

- `XmpSerializer`: Serializes XMP metadata to RDF/XML
- Generates XMP Packet format with `<?xpacket>` wrapper

### Node Types (`node.rs`)

- `SimpleNode`: Leaf nodes with values
- `ArrayNode`: Arrays (Ordered, Unordered, Alternate, AltText)
- `StructureNode`: Nested structures
- All nodes support qualifiers

### Namespace Management (`namespace.rs`)

- `NamespaceMap`: Manages namespace URI to prefix mappings
- Supports dynamic registration and lookup

## Files Module

The files module (`src/files/`) provides file format support:

### Handler Trait (`handler.rs`)

- `FileHandler`: Trait for file format handlers
- Methods: `can_handle`, `read_xmp`, `write_xmp`, `format_name`, `extensions`

### Registry (`registry.rs`)

- `HandlerRegistry`: Manages and detects file handlers
- Automatic format detection based on file signatures

### Format Handlers (`formats/`)

- **JPEG**: APP1 segment for XMP
- **PNG**: iTXt chunk for XMP
- **TIFF**: IFD tags for XMP
- **MP3**: ID3v2 PRIV frame for XMP
- **GIF**: Application Extension for XMP
- **MP4**: UUID box for XMP

## Design Principles

### Memory Safety

- Uses Rust's ownership system to ensure memory safety
- No unsafe code blocks
- Safe handling of file I/O

### Platform Compatibility

- Uses `Read + Seek` and `Write` traits for platform-agnostic I/O
- Supports native platforms (macOS, Linux, Windows, iOS, Android)
- Supports WebAssembly via in-memory I/O

### Minimal Dependencies

- Only depends on `quick-xml` for XML parsing
- No dependencies on image/audio/video decoding libraries
- Implements file format metadata parsing ourselves

### Modular Design

- Clear separation between core XMP functionality and file format handling
- Feature flags for optional file format support
- Easy to extend with new file formats

## Extension Points

### Adding a New File Format

1. Implement the `FileHandler` trait
2. Add handler to `HandlerRegistry`
3. Add feature flag in `Cargo.toml`
4. Register handler in `register_defaults`

### Adding New XMP Features

1. Extend `XmpMeta` with new methods
2. Update parser/serializer if needed
3. Add tests
4. Update documentation

