# PixChisel

PixChisel is a free, privacy-first desktop utility for converting, compressing,
and resizing images on macOS and Windows.

> Convert. Compress. Resize. Locally.

PixChisel is designed around a short workflow: add images, adjust options,
process, and review the result. Image data stays on the user's device. The app
has no account system, backend, cloud storage, or telemetry by default.

## Status

The repository contains the local import queue, orientation-aware thumbnails,
validated transformation settings, and an in-memory native transformation
pipeline. Final output writing and batch processing are not implemented yet.

## Technology

- Tauri 2 and Rust for native capabilities and image processing
- React and TypeScript for the interface and local UI state
- Vite for frontend development and builds
- npm for JavaScript package management

## Prerequisites

Install the current prerequisites from the
[Tauri setup guide](https://v2.tauri.app/start/prerequisites/) for your operating
system, plus:

- Node.js with npm
- Rust with Cargo

## Development

```sh
npm install
npm run tauri dev
```

Run only the browser-based UI during interface work:

```sh
npm run dev
```

## Validation

```sh
npm test
npm run build
npm run version:check
cargo check --manifest-path src-tauri/Cargo.toml --locked
```

## Versioning

PixChisel uses semantic versions in `MAJOR.MINOR.PATCH` form. `package.json` is
the canonical version source; Tauri and Cargo version fields are synchronized
from it.

Prepare an intentional pre-1.0 release with one of:

```sh
npm run release:patch
npm run release:minor
```

The major command also exists for the eventual 1.0 release:

```sh
npm run release:major
```

These commands update version metadata but deliberately create no Git tag or
release. Use `npm run version:check` to verify alignment, or
`npm run version:sync` after intentionally editing the canonical version.

## Project documentation

- [Project rules](docs/PROJECT_RULES.md)
- [Product specification](docs/PRODUCT_SPEC.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)

## Privacy principles

- Images are processed locally and never uploaded.
- The application requires no account or backend.
- Telemetry and analytics are absent by default.
- Network-dependent product features are outside the core architecture.

## License

A license has not yet been selected. Choose one before the first public release.
