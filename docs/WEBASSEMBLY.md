# WebAssembly JavaScript Integration Guide

This guide explains how to use xmpkit in JavaScript/TypeScript applications through WebAssembly.

## Overview

xmpkit can be compiled to WebAssembly and used in web browsers or Node.js. Since Wasm cannot access the file system directly, all operations work with file data in memory.

## Why wasm-bindgen?

You might wonder: "Can't I just compile to `wasm32-unknown-unknown` and use it directly?"

Technically yes, but `wasm-bindgen` makes it much easier:
- **Without wasm-bindgen**: You need to manually manage WebAssembly memory, pass raw pointers, handle type conversions, and write low-level bindings
- **With wasm-bindgen**: Automatic type conversion, memory management, and clean JavaScript APIs

According to the [MDN Rust to WebAssembly guide](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Rust_to_Wasm), `wasm-bindgen` is the recommended approach for Rust/WebAssembly integration as it significantly simplifies the development process.

If you prefer to use raw WebAssembly without wasm-bindgen, see the [Raw WebAssembly Usage](#raw-webassembly-usage-without-wasm-bindgen) section below.

## Setup

### Method 1: Use Built-in Wasm Bindings (Recommended)

xmpkit includes built-in wasm-bindgen bindings. Simply enable the `wasm` feature:

**1. Create a new crate:**

```bash
cargo new --lib my-wasm-app
cd my-wasm-app
```

**2. Configure Cargo.toml:**

```toml
[package]
name = "my-wasm-app"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
xmpkit = { version = "0.1.0", features = ["wasm"] }
```

**3. Re-export bindings in `src/lib.rs`:**

```rust
pub use xmpkit::wasm::*;
```

**4. Build:**

```bash
wasm-pack build --target web --out-dir pkg
```

That's it! The following classes and functions are now available:

**Classes:**
- `XmpFile` - Read/write XMP from file data
- `XmpMeta` - Parse and manipulate XMP metadata
- `ReadOptions` - Configure file reading options

**Functions:**
- `register_namespace(uri, prefix)` - Register custom namespace
- `namespace_uri(namespace)` / `namespace_prefix(namespace)` - Get built-in namespace URI/prefix
- `is_namespace_registered(uri)` - Check if namespace is registered
- `get_all_registered_namespaces()` - Get all registered namespaces

### Method 2: Create Custom Bindings

If you need custom bindings or want more control, create your own binding crate:

**1. Create a new crate:**

```bash
cargo new --lib xmpkit-wasm
cd xmpkit-wasm
```

**2. Configure Cargo.toml:**

```toml
[package]
name = "xmpkit-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
xmpkit = { path = "../xmpkit" }
wasm-bindgen = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

**3. Create custom bindings in `src/lib.rs`:

```rust
use wasm_bindgen::prelude::*;
use xmpkit::{XmpFile, XmpMeta, XmpValue, ReadOptions};
use xmpkit::core::namespace::ns;

#[wasm_bindgen]
pub fn read_xmp(data: &[u8]) -> Result<String, JsValue> {
    // Read-only mode: efficient, doesn't store file data in memory
    let mut file = XmpFile::new();
    file.from_bytes(data)
        .map_err(|e| JsValue::from_str(&format!("Failed to read file: {}", e)))?;
    
    let meta = file.get_xmp()
        .ok_or_else(|| JsValue::from_str("No XMP metadata found"))?;
    
    let mut result = String::new();
    if let Some(creator_tool) = meta.get_property(ns::XMP, "CreatorTool") {
        if let XmpValue::String(value) = creator_tool {
            result.push_str(&format!("CreatorTool: {}\n", value));
        }
    }
    Ok(result)
}

#[wasm_bindgen]
pub fn write_xmp(data: &[u8], creator_tool: &str) -> Result<Vec<u8>, JsValue> {
    // Read-write mode: use for_update() to enable writing
    let mut file = XmpFile::new();
    file.from_bytes_with(data, ReadOptions::default().for_update())
        .map_err(|e| JsValue::from_str(&format!("Failed to read file: {}", e)))?;
    
    let mut meta = file.get_xmp().cloned().unwrap_or_else(XmpMeta::new);
    meta.set_property(
        ns::XMP,
        "CreatorTool",
        XmpValue::String(creator_tool.to_string()),
    )
    .map_err(|e| JsValue::from_str(&format!("Failed to set property: {}", e)))?;
    
    file.put_xmp(meta);
    file.write_to_bytes()
        .map_err(|e| JsValue::from_str(&format!("Failed to write file: {}", e)))
}
```

### 4. Build with wasm-pack

```bash
# Install wasm-pack if needed
cargo install wasm-pack

# Build for web browsers
wasm-pack build --target web --out-dir pkg

# Or for Node.js
wasm-pack build --target nodejs --out-dir pkg
```

## JavaScript Usage

### Browser Usage

```html
<!DOCTYPE html>
<html>
<head>
    <title>XMPKit Wasm Example</title>
</head>
<body>
    <input type="file" id="fileInput" accept="image/*">
    <button onclick="readXMP()">Read XMP</button>
    <button onclick="writeXMP()">Write XMP</button>
    <pre id="output"></pre>

    <script type="module">
        import init, { XmpFile, XmpMeta, ReadOptions } from './pkg/my_wasm_app.js';

        let wasmReady = false;
        
        async function initWasm() {
            await init();
            wasmReady = true;
        }
        
        initWasm();

        window.readXMP = async function() {
            if (!wasmReady) return alert('Wasm not ready');
            
            const fileInput = document.getElementById('fileInput');
            const file = fileInput.files[0];
            if (!file) return alert('Please select a file');

            const arrayBuffer = await file.arrayBuffer();
            const uint8Array = new Uint8Array(arrayBuffer);
            
            try {
                // Read-only mode (memory efficient)
                const xmpFile = new XmpFile();
                xmpFile.from_bytes(uint8Array);
                
                const meta = xmpFile.get_xmp();
                if (meta) {
                    const creatorTool = meta.get_property("http://ns.adobe.com/xap/1.0/", "CreatorTool");
                    document.getElementById('output').textContent = 
                        `CreatorTool: ${creatorTool || 'Not found'}\n` +
                        `XMP Packet:\n${meta.serialize_packet()}`;
                } else {
                    document.getElementById('output').textContent = 'No XMP metadata found';
                }
            } catch (error) {
                document.getElementById('output').textContent = 'Error: ' + error;
            }
        };

        window.writeXMP = async function() {
            if (!wasmReady) return alert('Wasm not ready');
            
            const fileInput = document.getElementById('fileInput');
            const file = fileInput.files[0];
            if (!file) return alert('Please select a file');

            const arrayBuffer = await file.arrayBuffer();
            const uint8Array = new Uint8Array(arrayBuffer);
            
            try {
                // Read-write mode: use for_update() for writing
                const xmpFile = new XmpFile();
                const options = new ReadOptions();
                options.for_update();  // Required for write_to_bytes()
                xmpFile.from_bytes_with(uint8Array, options);
                
                // Get or create metadata
                let meta = xmpFile.get_xmp();
                if (!meta) {
                    meta = new XmpMeta();
                }
                
                // Set properties
                meta.set_property("http://ns.adobe.com/xap/1.0/", "CreatorTool", "MyApp");
                xmpFile.put_xmp(meta);
                
                // Write to bytes
                const modifiedData = xmpFile.write_to_bytes();
                
                // Download modified file
                const blob = new Blob([modifiedData], { type: file.type });
                const url = URL.createObjectURL(blob);
                const a = document.createElement('a');
                a.href = url;
                a.download = 'modified_' + file.name;
                a.click();
                URL.revokeObjectURL(url);
            } catch (error) {
                alert('Error: ' + error);
            }
        };
    </script>
</body>
</html>
```

### Node.js Usage

```javascript
const wasm = require('./pkg/my_wasm_app.js');
const fs = require('fs');

async function main() {
    await wasm.default(); // Initialize Wasm module
    
    const { XmpFile, XmpMeta, ReadOptions } = wasm;
    
    // Read file
    const fileData = fs.readFileSync('image.jpg');
    const uint8Array = new Uint8Array(fileData);
    
    // Read XMP (read-only mode)
    try {
        const xmpFile = new XmpFile();
        xmpFile.from_bytes(uint8Array);
        
        const meta = xmpFile.get_xmp();
        if (meta) {
            console.log('CreatorTool:', meta.get_property("http://ns.adobe.com/xap/1.0/", "CreatorTool"));
        }
    } catch (error) {
        console.error('Error reading XMP:', error);
    }
    
    // Write XMP (read-write mode)
    try {
        const xmpFile = new XmpFile();
        const options = new ReadOptions();
        options.for_update();  // Required for write_to_bytes()
        xmpFile.from_bytes_with(uint8Array, options);
        
        let meta = xmpFile.get_xmp();
        if (!meta) meta = new XmpMeta();
        
        meta.set_property("http://ns.adobe.com/xap/1.0/", "CreatorTool", "MyApp");
        xmpFile.put_xmp(meta);
        
        const modifiedData = xmpFile.write_to_bytes();
        fs.writeFileSync('output.jpg', Buffer.from(modifiedData));
        console.log('File written successfully');
    } catch (error) {
        console.error('Error writing XMP:', error);
    }
}

main();
```

### TypeScript Usage

```typescript
import init, { XmpFile, XmpMeta, ReadOptions } from './pkg/my_wasm_app';

async function readXmp(file: File): Promise<string | null> {
    await init();
    
    const arrayBuffer = await file.arrayBuffer();
    const uint8Array = new Uint8Array(arrayBuffer);
    
    const xmpFile = new XmpFile();
    xmpFile.from_bytes(uint8Array);
    
    const meta = xmpFile.get_xmp();
    return meta?.get_property("http://ns.adobe.com/xap/1.0/", "CreatorTool") ?? null;
}

async function writeXmp(file: File, creatorTool: string): Promise<Uint8Array> {
    await init();
    
    const arrayBuffer = await file.arrayBuffer();
    const uint8Array = new Uint8Array(arrayBuffer);
    
    // Use for_update() for write operations
    const xmpFile = new XmpFile();
    const options = new ReadOptions();
    options.for_update();
    xmpFile.from_bytes_with(uint8Array, options);
    
    let meta = xmpFile.get_xmp();
    if (!meta) meta = new XmpMeta();
    
    meta.set_property("http://ns.adobe.com/xap/1.0/", "CreatorTool", creatorTool);
    xmpFile.put_xmp(meta);
    
    return xmpFile.write_to_bytes();
}
```

## API Reference

### Class: `XmpFile`

Main class for reading/writing XMP from file data.

#### `new XmpFile()`

Creates a new XmpFile instance.

#### `from_bytes(data: Uint8Array): void`

Load file data in **read-only mode** (memory efficient, cannot write).

#### `from_bytes_with(data: Uint8Array, options: ReadOptions): void`

Load file data with options. Use `options.for_update()` to enable writing.

#### `get_xmp(): XmpMeta | null`

Get XMP metadata from file.

#### `put_xmp(meta: XmpMeta): void`

Set XMP metadata to file.

#### `write_to_bytes(): Uint8Array`

Write file with XMP to bytes. **Requires `for_update()` option when loading.**

### Class: `XmpMeta`

Class for parsing and manipulating XMP metadata.

#### `new XmpMeta()`

Creates a new empty XmpMeta instance.

#### `XmpMeta.parse(xmpPacket: string): XmpMeta`

Parse XMP packet XML string.

#### `get_property(namespace: string, property: string): string | null`

Get a property value.

#### `set_property(namespace: string, property: string, value: string): void`

Set a property value.

#### `delete_property(namespace: string, property: string): void`

Delete a property.

#### `has_property(namespace: string, property: string): boolean`

Check if a property exists.

#### `serialize(): string`

Serialize to RDF/XML string.

#### `serialize_packet(): string`

Serialize to XMP packet string (with `<?xpacket>` wrapper).

### Class: `ReadOptions`

Options for reading files.

#### `new ReadOptions()`

Creates default options.

#### `for_update(): void`

Enable read-write mode. **Required for `write_to_bytes()`.**

#### `use_packet_scanning(): void`

Force packet scanning mode.

#### `use_smart_handler(): void`

Require smart handler.

### Functions

#### `register_namespace(uri: string, prefix: string): void`

Register a custom namespace.

#### `is_namespace_registered(uri: string): boolean`

Check if namespace is registered.

#### `get_all_registered_namespaces(): object`

Get all registered namespaces as `{ uri: prefix }` object.

## Building and Deployment

### Development Build

```bash
wasm-pack build --target web --dev
```

### Production Build

```bash
wasm-pack build --target web --release
```

### Optimize Size

```bash
# Install wasm-opt
npm install -g wasm-opt

# Optimize
wasm-opt pkg/xmpkit_wasm_bg.wasm -o pkg/xmpkit_wasm_bg.wasm -Oz
```

## Raw WebAssembly Usage (Without wasm-bindgen)

If you want to avoid wasm-bindgen and use raw WebAssembly, you can compile directly:

```bash
# Build to wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release

# The output will be in target/wasm32-unknown-unknown/release/xmpkit.wasm
```

However, you'll need to:
1. Manually export functions using `#[no_mangle]` and `extern "C"`
2. Handle memory allocation/deallocation manually
3. Pass data through WebAssembly memory buffers
4. Write low-level JavaScript bindings using the WebAssembly JavaScript API

**Example (simplified, not recommended for production):**

```rust
// In your Rust code - requires manual memory management
use std::alloc::{alloc, dealloc, Layout};

#[no_mangle]
pub extern "C" fn process_xmp(data_ptr: *const u8, data_len: usize) -> *mut u8 {
    // Much more complex: manual memory management, pointer handling, etc.
    // This is why wasm-bindgen is recommended
}
```

```javascript
// In JavaScript - much more complex
const wasmModule = await WebAssembly.instantiateStreaming(fetch('xmpkit.wasm'));
const memory = wasmModule.instance.exports.memory;
const processXmp = wasmModule.instance.exports.process_xmp;
// Manual memory management, pointer handling, type conversions, etc.
```

**Recommendation**: Use wasm-bindgen for a much simpler and safer development experience. The setup overhead is minimal compared to the complexity of manual WebAssembly memory management. According to the [MDN guide](https://developer.mozilla.org/en-US/docs/WebAssembly/Guides/Rust_to_Wasm), wasm-bindgen is the standard approach for Rust/WebAssembly integration.

## Limitations

- File operations are memory-based only (no file system access)
- Large files may consume significant memory
- Error handling returns JavaScript strings/errors

## Examples

See `examples/wasm_bindings.rs` for a complete wasm-bindgen implementation example.
