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

That's it! The bindings (`read_xmp`, `write_xmp`, `parse_xmp_packet`) are now available.

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
use xmpkit::{XmpFile, XmpMeta, XmpValue};
use xmpkit::core::namespace::ns;

#[wasm_bindgen]
pub fn read_xmp(data: &[u8]) -> Result<String, JsValue> {
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
    let mut file = XmpFile::new();
    file.from_bytes(data)
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
        import init, { read_xmp, write_xmp } from './pkg/xmpkit_wasm.js';

        let wasmModule;
        
        async function initWasm() {
            wasmModule = await init();
        }
        
        initWasm();

        window.readXMP = async function() {
            const fileInput = document.getElementById('fileInput');
            const file = fileInput.files[0];
            if (!file) {
                alert('Please select a file');
                return;
            }

            const arrayBuffer = await file.arrayBuffer();
            const uint8Array = new Uint8Array(arrayBuffer);
            
            try {
                const result = read_xmp(uint8Array);
                document.getElementById('output').textContent = result;
            } catch (error) {
                document.getElementById('output').textContent = 'Error: ' + error;
            }
        };

        window.writeXMP = async function() {
            const fileInput = document.getElementById('fileInput');
            const file = fileInput.files[0];
            if (!file) {
                alert('Please select a file');
                return;
            }

            const arrayBuffer = await file.arrayBuffer();
            const uint8Array = new Uint8Array(arrayBuffer);
            
            // Set properties
            const properties = JSON.stringify({
                creatorTool: "MyApp",
                title: "My Image"
            });
            
            try {
                const modifiedData = write_xmp(uint8Array, properties);
                
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
const wasm = require('./pkg/xmpkit_wasm.js');
const fs = require('fs');

async function main() {
    await wasm.default(); // Initialize Wasm module
    
    // Read file
    const fileData = fs.readFileSync('image.jpg');
    const uint8Array = new Uint8Array(fileData);
    
    // Read XMP
    try {
        const xmpJson = wasm.read_xmp(uint8Array);
        console.log('XMP Data:', JSON.parse(xmpJson));
    } catch (error) {
        console.error('Error reading XMP:', error);
    }
    
    // Write XMP
    const properties = JSON.stringify({
        creatorTool: "MyApp",
        title: "My Image"
    });
    
    try {
        const modifiedData = wasm.write_xmp(uint8Array, properties);
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
import init, { read_xmp, write_xmp } from './pkg/xmpkit_wasm';

interface XMPProperties {
    creatorTool?: string;
    createDate?: string;
    title?: string;
    description?: string;
}

async function processImage(file: File): Promise<Uint8Array> {
    // Initialize Wasm module
    await init();
    
    // Read file
    const arrayBuffer = await file.arrayBuffer();
    const uint8Array = new Uint8Array(arrayBuffer);
    
    // Read existing XMP
    const xmpJson = read_xmp(uint8Array);
    const properties: XMPProperties = JSON.parse(xmpJson);
    console.log('Current XMP:', properties);
    
    // Modify properties
    const newProperties: XMPProperties = {
        ...properties,
        creatorTool: "MyApp",
        title: "Updated Title"
    };
    
    // Write XMP
    const modifiedData = write_xmp(uint8Array, JSON.stringify(newProperties));
    return modifiedData;
}
```

## API Reference

### `read_xmp(data: Uint8Array): string`

Reads XMP metadata from file data and returns a JSON string with properties.

**Parameters:**
- `data`: File data as `Uint8Array`

**Returns:**
- JSON string with XMP properties

**Example:**
```javascript
const fileData = new Uint8Array(/* file bytes */);
const xmpJson = read_xmp(fileData);
const properties = JSON.parse(xmpJson);
console.log(properties.creatorTool);
```

### `write_xmp(data: Uint8Array, properties: string): Uint8Array`

Writes XMP metadata to file data and returns modified file data.

**Parameters:**
- `data`: Original file data as `Uint8Array`
- `properties`: JSON string with properties to set

**Returns:**
- Modified file data as `Uint8Array`

**Example:**
```javascript
const fileData = new Uint8Array(/* file bytes */);
const properties = JSON.stringify({
    creatorTool: "MyApp",
    title: "My Image"
});
const modifiedData = write_xmp(fileData, properties);
```

### `parse_xmp_packet(xmp_packet: string): string`

Parses an XMP packet XML string and returns extracted properties as JSON.

**Parameters:**
- `xmp_packet`: XMP packet XML string

**Returns:**
- JSON string with extracted properties

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

