# Building for OpenHarmony/HarmonyOS

This guide explains how to build xmpkit for OpenHarmony/HarmonyOS platforms.

## Prerequisites

1. Install Rust toolchain:
   ```bash
   rustup target add aarch64-unknown-linux-ohos
   rustup target add armv7-unknown-linux-ohos
   rustup target add x86_64-unknown-linux-ohos
   ```

2. Install HarmonyOS NDK and set environment variable:
   ```bash
   export OHOS_NDK_HOME=/path/to/openharmony/ndk
   ```

3. Install `ohrs` CLI tool:
   ```bash
   cargo install ohrs
   ```

## Building

### Using ohrs (Recommended)

The `ohrs` tool simplifies the build process:

```bash
# Build for all architectures
ohrs build --features ohos

# Build for specific architecture
ohrs build --features ohos --arch aarch64

# Build release version
ohrs build --features ohos --release
```

The build artifacts will be placed in the `dist` directory:
- `dist/arm64-v8a/libxmpkit.so` - ARM64 version
- `dist/armeabi-v7a/libxmpkit.so` - ARMv7 version
- `dist/x86_64/libxmpkit.so` - x86_64 version

### Using cargo directly

You can also build directly with cargo:

```bash
# Build for ARM64
cargo build --features ohos --lib --release --target aarch64-unknown-linux-ohos

# Build for ARMv7
cargo build --features ohos --lib --release --target armv7-unknown-linux-ohos

# Build for x86_64
cargo build --features ohos --lib --release --target x86_64-unknown-linux-ohos
```

The built `.so` files will be in `target/{target-triple}/release/`.

## Integration

1. Copy the `.so` files to your OpenHarmony project's `libs` directory:
   ```
   libs/
   ├── arm64-v8a/
   │   └── libxmpkit.so
   ├── armeabi-v7a/
   │   └── libxmpkit.so
   └── x86_64/
       └── libxmpkit.so
   ```

2. Use in ArkTS:
   ```typescript
   import { XmpFile, XmpMeta } from 'libxmpkit.so';
   
   const file = new XmpFile();
   file.fromBytes(fileBytes);
   const meta = file.getXmp();
   ```

## Troubleshooting

### Missing environment variables

If you see errors about missing environment variables when using `ohrs build`, ensure:
- `OHOS_NDK_HOME` is set correctly
- You have the latest version of `ohrs`: `cargo install --force ohrs`
- Create `.cargo/config.toml` to set environment variables:
  ```toml
  [env]
  OHOS_NDK_HOME = { value = "/path/to/openharmony", force = false }
  ```

### Linker errors

If you encounter linker errors, ensure:
- The HarmonyOS NDK is properly installed
- The correct toolchain is selected
- Environment variables are set correctly

