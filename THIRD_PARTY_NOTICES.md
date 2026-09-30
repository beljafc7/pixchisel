# Third-Party Notices

## PixChisel

PixChisel is free and open-source software.

Copyright © 2026 Johny Fandino. PixChisel is licensed under the
[GNU General Public License v3.0 only](LICENSE). The third-party licenses below
continue to apply to their respective components.

## Third-party dependencies

PixChisel distributions include or are built from third-party software. Each
component remains owned by its respective owner and licensed under its own
terms. The production/runtime dependency graph was inspected from
`package-lock.json`, `src-tauri/Cargo.lock`, installed package metadata, and the
enabled Cargo features.

Key shipped components include:

| Component | Locked version(s) | Role | License |
| --- | --- | --- | --- |
| Tauri / `@tauri-apps/api` | 2.11.5 / 2.11.1 | Desktop runtime and frontend/native bridge | MIT or Apache-2.0 |
| Tauri dialog plugin / `@tauri-apps/plugin-dialog` | 2.7.2 | Native file and folder dialogs | MIT or Apache-2.0 |
| React / React DOM / Scheduler | 19.2.8 / 19.2.8 / 0.27.0 | User interface runtime | MIT |
| `image` / image-rs | 0.25.10 | Image decoding plus PNG and lossless WebP encoding | MIT or Apache-2.0 |
| `color_quant` | 2.0.0 | RGBA palette quantization for Maximum PNG compression | MIT |
| `jpeg-encoder` | 0.7.1 | Progressive JPEG encoding with optimized Huffman tables | (MIT or Apache-2.0) and IJG |
| Oxipng | 10.2.0 | Lossless in-memory PNG optimization | MIT |
| `libdeflater` / `libdeflate-sys` | 1.25.2 / 1.25.2 | Rust wrapper and build integration for the Oxipng compression backend | Apache-2.0 |
| libdeflate | Vendored by `libdeflate-sys` 1.25.2 | Statically built compression implementation used by Oxipng | MIT |
| `libwebp-sys` | 0.9.6 | Rust FFI/build integration for libwebp | MIT |
| libwebp | Vendored by `libwebp-sys` 0.9.6 | Statically linked WebP codec implementation | BSD 3-Clause |
| Serde / `serde_json` | 1.0.229 / 1.0.151 | Native command serialization | MIT or Apache-2.0 |

PixChisel also contains transitive dependencies of these components. Their
license metadata is recorded in the JavaScript and Rust lockfiles and package
sources. Platform webviews and operating-system frameworks are supplied by the
user's operating system and are subject to their platform terms.

The application bundle includes this notice, the PixChisel GPL license, the
PixChisel trademark notice, the generated Rust dependency license inventory in
[`THIRD_PARTY_LICENSES.html`](THIRD_PARTY_LICENSES.html), and the generated npm
production dependency license inventory in
[`THIRD_PARTY_NPM_LICENSES.html`](THIRD_PARTY_NPM_LICENSES.html), plus the
vendored [libwebp](licenses/libwebp-COPYING.txt) and
[libdeflate](licenses/libdeflate-COPYING.txt) notices required by their bundled
native code.

## Distribution requirements

Before publishing any installer or application bundle, the release process must:

1. regenerate the version-specific inventories from the locked production
   dependency graphs with `npm run licenses:generate`;
2. include this notice, the PixChisel license, and the full license texts and
   attribution notices required by every distributed third-party component and
   transitive dependency;
3. include the BSD notice required for the statically linked libwebp sources;
4. include the IJG notice and the selected MIT or Apache-2.0 terms required by
   `jpeg-encoder`;
5. include the MIT notices for `color_quant`, Oxipng, and the vendored
   libdeflate code, plus the Apache-2.0 notices required by the `libdeflater`
   and `libdeflate-sys` Rust crates;
6. preserve any copyright, attribution, and notice files required by the chosen
   MIT/Apache-2.0 license options; and
7. review target-specific macOS and Windows packages separately because their
   dependency sets may differ.

This repository summary is not a substitute for the complete version-specific
notice bundle that must accompany a public release.
