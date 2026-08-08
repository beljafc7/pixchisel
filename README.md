# PixChisel

PixChisel is a free, privacy-first desktop utility for converting, compressing,
and resizing images on macOS and Windows.

> Convert. Compress. Resize. Locally.

PixChisel is designed around a short workflow: add images, adjust options,
process, and review the result. Image data stays on the user's device. The app
has no account system, backend, cloud storage, or telemetry by default.

## Status

The repository currently contains the initial project foundation and a minimal
application shell. Image import and processing are intentionally not implemented
yet.

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
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```

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
