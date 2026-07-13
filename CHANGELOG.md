# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.5](https://github.com/cavivie/xmpkit/compare/v0.1.4...v0.1.5) - 2026-07-13

### Fixed

- serialize metadata deterministically ([#130](https://github.com/cavivie/xmpkit/pull/130))
- keep nested description attributes scoped ([#129](https://github.com/cavivie/xmpkit/pull/129))

### Other

- *(deps)* update vue ecosystem dependencies
- *(deps)* update playwright
- *(deps)* update eslint playwright plugin
- *(deps)* update element plus
- *(deps)* update lopdf to 0.44
- *(deps)* update web build tools
- run dependabot monthly ([#122](https://github.com/cavivie/xmpkit/pull/122))
- *(deps)* (deps): update lopdf requirement from 0.41 to 0.42 ([#111](https://github.com/cavivie/xmpkit/pull/111))
- *(ci)* (deps): bump actions/checkout from 6 to 7 ([#110](https://github.com/cavivie/xmpkit/pull/110))

## [0.1.4](https://github.com/cavivie/xmpkit/compare/v0.1.3...v0.1.4) - 2026-06-16

### Added

- *(web)* migrate to Vite+

### Fixed

- *(svg)* adapt attribute normalization for quick-xml 0.40
- *(web)* use exported wasm options type
- *(core)* correct EXIF Aux prefix and add EXIF 2.32 Extension namespace
- *(core)* support attributes-as-properties on nested elements and array items
- *(core)* fix nested structure parsing and metadata bugs.

### Other

- add semantic PR title check
- Fix RDF Description parsing and resolve clippy warnings
- *(deps)* (deps): bump the vue-ecosystem group across 1 directory with 3 updates
- *(deps)* (deps): update quick-xml requirement from 0.39 to 0.40
- *(deps)* (deps): bump the other-dependencies group across 1 directory with 2 updates
- *(deps)* (deps-dev): bump the testing group across 1 directory with 3 updates
- *(deps)* (deps-dev): bump the build-tools group across 1 directory with 3 updates
- *(deps)* (deps): update lopdf requirement from 0.40 to 0.41
- *(deps)* (deps-dev): bump the linting group across 1 directory with 3 updates
- Refactor RDF parser in parser.rs
- *(core)* fix clippy and fmt lint warnings
- *(ci)* (deps): bump actions/upload-pages-artifact from 3 to 5
- *(deps)* (deps): bump the build-tools group in /web with 5 updates ([#82](https://github.com/cavivie/xmpkit/pull/82))
- *(deps)* (deps-dev): bump the testing group across 1 directory with 3 updates ([#86](https://github.com/cavivie/xmpkit/pull/86))
- *(deps)* (deps): bump the vue-ecosystem group in /web with 3 updates
- *(ci)* (deps): bump actions/deploy-pages from 4 to 5
- *(ci)* (deps): bump openharmony-rs/setup-ohos-sdk from 0.2 to 1.0
- *(deps)* (deps-dev): bump the testing group in /web with 2 updates
- *(deps)* (deps-dev): bump eslint-plugin-playwright
- *(deps)* (deps): bump element-plus
- *(ci)* (deps): bump actions/configure-pages from 5 to 6
- *(deps)* (deps): update lopdf requirement from 0.39 to 0.40

## [0.1.3](https://github.com/cavivie/xmpkit/compare/v0.1.2...v0.1.3) - 2026-03-20

### Added

- enhance XmpMeta serialization with namespace support and add namespace map retrieval in parser

### Fixed

- *(dep)* add wasm_js feature of lopdf(0.37 and higher)
- *(ci)* add permissions for contents in publish workflow ([#58](https://github.com/cavivie/xmpkit/pull/58))

### Other

- *(deps)* (deps-dev): bump the testing group across 1 directory with 3 updates ([#73](https://github.com/cavivie/xmpkit/pull/73))
- *(deps)* (deps-dev): bump the build-tools group across 1 directory with 5 updates
- *(deps)* (deps-dev): bump the linting group across 1 directory with 3 updates
- *(deps)* (deps): bump the vue-ecosystem group across 1 directory with 2 updates
- *(deps)* (deps): update lopdf requirement from 0.38 to 0.39
- *(deps)* (deps): update quick-xml requirement from 0.38 to 0.39
- *(deps)* (deps): bump element-plus
- Fix formatting
- Fix infinite loop when reading invalid MP4 boxes
- Add serde::Serialize to XmpValue
- Add a way to get all properties

## [0.1.2](https://github.com/cavivie/xmpkit/compare/v0.1.1...v0.1.2) - 2025-12-15

### Added

- refactor BMFF module and add HEIF/AVIF support ([#57](https://github.com/cavivie/xmpkit/pull/57))
- *(files)* add RIFF module with WAV and AVI support
- *(svg)* add SVG file format support
- *(psd)* add PSD/PSB file format support
- *(webp)* add WebP file format support
- *(mpeg4)* add native metadata reconciliation
- *(mpeg4)* add .mov extension support
- add PDF file format support using lopdf

### Fixed

- *(psd)* use is_multiple_of() for clippy 1.91
- add for_update() option to WASM bindings and update docs
- parse XMP without reading whole file into memory
- *(tiff)* correct XMP data offset calculation and write order
- *(mp3)* update tag size when writing XMP to existing ID3v2 tag
- remove extra byte from GIF magic trailer
- mainly fix incorrect title logo url address

### Other

- *(files)* optimize can_handle following C++ SDK patterns
- rename mp4 feature to mpeg4
- Revert "refactor: rename mp4 feature to mpeg4"
- rename mp4 feature to mpeg4
- *(deps)* (deps): bump vue-i18n in /web in the vue-ecosystem group
- *(deps)* (deps-dev): bump vue-tsc in /web in the build-tools group
- *(deps)* (deps-dev): bump the testing group in /web with 2 updates
- *(deps)* (deps-dev): bump eslint-plugin-vue
- *(deps)* (deps): bump element-plus
- optimize dependabot configuration
- add Cargo.lock to .gitignore (library best practice)
- *(deps)* bump eslint-plugin-playwright from 2.3.0 to 2.4.0 in /web
- *(deps)* bump @types/node from 24.10.1 to 24.10.2 in /web
- *(deps)* bump @vitest/eslint-plugin from 1.5.0 to 1.5.2 in /web
- *(deps)* bump vitest from 4.0.13 to 4.0.15 in /web
- *(deps)* bump vite from 7.2.4 to 7.2.7 in /web
- *(deps)* (deps): bump criterion from 0.8.0 to 0.8.1
- *(deps)* (deps): bump criterion from 0.7.0 to 0.8.0
- *(deps)* bump vue from 3.5.24 to 3.5.25 in /web
- *(deps)* bump vue-i18n from 11.1.12 to 11.2.1 in /web
- *(deps)* bump vitest from 4.0.10 to 4.0.13 in /web
- *(deps)* bump vite from 7.2.2 to 7.2.4 in /web
- *(deps)* bump @vitest/eslint-plugin from 1.4.3 to 1.5.0 in /web
- add comprehensive tests for all file formats
- add tests to GIF module
- *(ci)* (deps): bump actions/checkout from 5 to 6
- update document ref and license copyright
- make quick start document more useful
- adapt wasm web ui for mobile devices

## [0.1.1](https://github.com/cavivie/xmpkit/compare/v0.1.0...v0.1.1) - 2025-11-19

### Fixed

- incorrect repository url in cargo toml

### Other

- bump criterion from 0.5 to 0.7
- *(deps)* bump vitest from 3.2.4 to 4.0.10 in /web
- *(deps)* bump @vitejs/plugin-vue from 6.0.1 to 6.0.2 in /web
- improve dependabot commit message format
- bump actions/upload-pages-artifact from 3 to 4
- Initial Commit
