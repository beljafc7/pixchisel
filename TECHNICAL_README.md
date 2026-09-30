# PixChisel Technical Documentation

This document contains development, validation, architecture, versioning, and
release information for PixChisel contributors.

## Project status

PixChisel 0.5.0 contains the complete local import-to-output workflow:
orientation-aware thumbnails, validated batch settings, native image
transformation, safe output writing, bounded processing, cancellation, results,
destination preflight, and native output-folder opening.

The macOS `x86_64` application and DMG have been validated locally. Windows and
native Apple Silicon builds require platform-specific runtime and installer QA.

## Technology

- Tauri 2 and Rust for native integration and image processing
- React and TypeScript for the interface and local state
- Vite for frontend development and production builds
- npm for JavaScript dependency management

## Prerequisites

Install the current prerequisites from the
[Tauri setup guide](https://v2.tauri.app/start/prerequisites/) for your operating
system, plus:

- Node.js with npm
- Rust with Cargo

## Development

Install dependencies and start the desktop application:

```sh
npm install
npm run tauri dev
```

Run only the browser-based interface during frontend work:

```sh
npm run dev
```

Create a production package for the current operating system:

```sh
npm run tauri build
```

## Validation

```sh
npm test
npm run build
npm run version:check
npm audit --omit=dev
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo audit --file src-tauri/Cargo.lock
```

## Versioning

PixChisel uses semantic versions in `MAJOR.MINOR.PATCH` form. `package.json` is
the canonical version source. Tauri and Cargo version fields are synchronized
from it.

Prepare an intentional pre-1.0 release with one of:

```sh
npm run release:patch
npm run release:minor
```

The major command is reserved for the eventual 1.0 release:

```sh
npm run release:major
```

These commands update version metadata but deliberately create no Git tag or
GitHub Release. Use `npm run version:check` to verify alignment or
`npm run version:sync` after intentionally editing the canonical version.

## Dependency licenses

Release packages include the PixChisel license, trademark notice, third-party
notice, generated Rust and npm dependency license inventories, and notices for
vendored native libraries.

Install `cargo-about` and regenerate the version-specific inventories before a
release:

```sh
cargo install --locked --features cli cargo-about --version 0.9.1
npm run licenses:generate
```

## Privacy and security architecture

- Images are processed locally and never uploaded.
- The application has no account, backend, cloud storage, analytics, or
  telemetry.
- Thumbnail sessions are cleaned automatically.
- The desktop webview uses a restrictive Content Security Policy.
- The asset protocol is limited to the application thumbnail cache.
- Native capabilities are restricted to required desktop operations.

Report suspected vulnerabilities privately through
[GitHub Security Advisories](https://github.com/beljafc7/pixchisel/security/advisories/new).

## Project documentation

- [Changelog](CHANGELOG.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Release guide](docs/RELEASING.md)
- [Project rules](docs/PROJECT_RULES.md)
- [Product specification](docs/PRODUCT_SPEC.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [Third-party notices](THIRD_PARTY_NOTICES.md)
- [Rust dependency licenses](THIRD_PARTY_LICENSES.html)
- [npm dependency licenses](THIRD_PARTY_NPM_LICENSES.html)

## License

PixChisel is free and open-source software licensed under the
[GNU General Public License v3.0 only](LICENSE). Distributed modified versions
must provide their corresponding source code under GPL v3.0.

The GPL does not grant permission to imply endorsement or use the PixChisel name
and logo for a modified product. See [TRADEMARKS.md](TRADEMARKS.md).
