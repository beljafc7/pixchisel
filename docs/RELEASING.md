# Release Guide

This guide covers the current manual public-release process. Signing,
notarization, and automated multi-platform publishing can be added later without
changing the application architecture.

## 1. Prepare the release

Confirm that the intended version is aligned across npm, Cargo, and Tauri:

```sh
npm run version:check
```

Install dependencies from the lockfile:

```sh
npm ci
```

Install the license inventory generator when it is not already available:

```sh
cargo install --locked --features cli cargo-about
```

Regenerate the production dependency license inventories:

```sh
npm run licenses:generate
```

Review changes to both generated inventory files before packaging.

## 2. Validate

```sh
npm test
npm run build
npm run version:check
npm audit --omit=dev
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo audit --file src-tauri/Cargo.lock
```

The release owner must also test the packaged application manually with JPEG,
PNG, and WebP inputs across Convert, Compress, and Resize. Test Create Copies,
Replace Originals, mixed-source output selection, cancellation, and output-folder
opening.

## 3. Build the platform package

Build on the operating system being distributed:

```sh
npm run tauri build
```

Tauri writes platform bundles below `src-tauri/target/release/bundle/`. Do not
publish an artifact for an operating system that has not completed runtime and
installer validation on that operating system.

The bundle includes the PixChisel GPL license, trademark notice, third-party
notice, and generated Rust and npm dependency license inventories.

## 4. Publish on GitHub

1. Commit the release metadata, changelog, generated license inventories, and
   application changes.
2. Create an annotated version tag using the `vMAJOR.MINOR.PATCH` convention.
3. Create a GitHub Release from that tag.
4. Copy the matching section from `CHANGELOG.md` into the release notes.
5. Upload only the packages validated for that release.
6. Label unsigned or unnotarized preview artifacts clearly.
7. Confirm that GitHub provides the source archive for the exact release tag.
8. Download the published artifact once and perform a final clean-install test.

## Current 0.5.0 scope

- macOS: the current package is `x86_64`; label it clearly as Intel/Rosetta and
  complete a packaged-app smoke test before upload.
- macOS Apple Silicon: native `aarch64` package is deferred.
- Windows: do not attach an installer until Windows runtime and installer QA are
  complete.
- Code signing and macOS notarization: deferred; describe the preview as
  unsigned until these are implemented.
- Release automation: deferred; use the manual process above.
