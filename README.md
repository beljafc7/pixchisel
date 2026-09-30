<p align="center">
  <img src="assets/branding/pixchisel-logo.png" alt="PixChisel" width="720">
</p>

<p align="center">
  <strong>Convert. Compress. Resize. Locally.</strong>
</p>

<p align="center">
  A free, privacy-first desktop image utility. No uploads, accounts, cloud
  storage, or telemetry.
</p>

<p align="center">
  <a href="https://github.com/beljafc7/pixchisel/releases">Download PixChisel</a>
  ·
  <a href="https://github.com/beljafc7/pixchisel/issues">Report an issue</a>
  ·
  <a href="LICENSE">GPL-3.0</a>
</p>

## What PixChisel does

- **Convert** JPEG, PNG, and WebP images into another supported format.
- **Compress** images with fidelity-first Standard or smaller Maximum output.
- **Resize** by width, height, percentage, or bounding box while preserving
  proportions and physical resolution metadata where supported.
- Process multiple images or complete folders in one local batch.
- Create non-destructive copies or intentionally replace original files.
- Keep image data on the computer at every stage of the workflow.

## Download

Download the latest available build from the
[GitHub Releases page](https://github.com/beljafc7/pixchisel/releases).

PixChisel 0.5.0 is the first public preview. The current macOS DMG is an Intel
`x86_64` build; Apple Silicon Macs can run it through Rosetta. A native Apple
Silicon build and Windows installer will follow after platform-specific
validation.

Early builds may not yet be signed or notarized. Your operating system can
therefore ask you to confirm that you trust the downloaded application.

## How it works

1. Add individual images or a folder.
2. Choose Convert, Compress, or Resize.
3. Adjust the task options.
4. Choose Create Copies or Replace Originals.
5. Process the batch and review the result locally.

When Create Copies is selected, images from one source location are written to
a `PixChisel Copies` subfolder. Batches containing images from multiple source
locations ask for one shared output folder.

## Privacy and security

PixChisel has no account system, backend, cloud storage, analytics, or telemetry.
Images are decoded and transformed locally. Temporary thumbnail sessions are
cleaned automatically and stale abandoned sessions are pruned when the app
starts.

The desktop webview uses a restrictive Content Security Policy, and native
capabilities are limited to the operations required by the application.

Please report security concerns privately through
[GitHub Security Advisories](https://github.com/beljafc7/pixchisel/security/advisories/new).
See [SECURITY.md](SECURITY.md) for the disclosure policy.

## Build from source

Install the current prerequisites from the
[Tauri setup guide](https://v2.tauri.app/start/prerequisites/), plus Node.js,
npm, Rust, and Cargo.

```sh
npm install
npm run tauri dev
```

Create a local production bundle with:

```sh
npm run tauri build
```

Run the validation suite with:

```sh
npm test
npm run build
npm run version:check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo check --manifest-path src-tauri/Cargo.toml --locked
```

## Technology

- Tauri 2 and Rust for native integration and image processing
- React and TypeScript for the interface
- Vite for frontend development and production builds

## Project documentation

- [Changelog](CHANGELOG.md)
- [Contributing](CONTRIBUTING.md)
- [Release guide](docs/RELEASING.md)
- [Product specification](docs/PRODUCT_SPEC.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [Third-party notices](THIRD_PARTY_NOTICES.md)
- [Rust dependency licenses](THIRD_PARTY_LICENSES.html)
- [npm dependency licenses](THIRD_PARTY_NPM_LICENSES.html)

## License

PixChisel is free and open-source software licensed under the
[GNU General Public License v3.0 only](LICENSE). You may use, study, modify, and
redistribute it under those terms. Distributed modified versions must provide
their corresponding source code under GPL v3.0.

The GPL does not grant permission to imply endorsement or use the PixChisel name
and logo for a modified product. See [TRADEMARKS.md](TRADEMARKS.md).
