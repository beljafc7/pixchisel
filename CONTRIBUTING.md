# Contributing to PixChisel

Thank you for helping improve PixChisel.

## Before starting

- Search existing issues before opening a new one.
- Use an issue to discuss substantial behavior or architecture changes first.
- Keep changes focused and avoid unrelated refactoring.
- Do not include user images, credentials, personal paths, or generated build
  output in a contribution.

## Development

Install the prerequisites listed in the
[Technical README](TECHNICAL_README.md), then run:

```sh
npm install
npm run tauri dev
```

Before submitting a pull request, run:

```sh
npm test
npm run build
npm run version:check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo check --manifest-path src-tauri/Cargo.toml --locked
```

## Pull requests

Describe the user-visible outcome, relevant implementation decisions, and the
validation performed. Add or update tests when behavior changes.

By contributing, you agree that your contribution is licensed under the
project's [GNU General Public License v3.0 only](LICENSE).
