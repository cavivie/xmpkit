# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3](https://github.com/cavivie/xmpkit/compare/v0.1.2...v0.1.3) - 2025-12-30

### Fixed

- *(ci)* add permissions for contents in publish workflow ([#58](https://github.com/cavivie/xmpkit/pull/58))

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
