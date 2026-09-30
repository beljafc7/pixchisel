# Project Rules

These rules define the constraints for PixChisel development. Changes that
conflict with them require an explicit product and architecture decision.

## Product boundaries

- PixChisel is a free, local-first desktop image utility for macOS and Windows.
- The primary flow is: add images, adjust options, process, review results.
- Images must never be uploaded to a server.
- The app must work fully offline.
- No account, authentication, backend, database, cloud storage, or analytics is
  part of the product.
- Telemetry is off by default. Adding telemetry requires an explicit future
  decision and must never compromise the privacy promise.
- The interface should feel like a focused native utility, not a web dashboard.

## Engineering ownership

### React and TypeScript

The frontend owns presentation, local UI state, user input, validation messages,
and display formatting. It may orchestrate native commands but must not perform
heavy image processing.

### Rust and Tauri

The native layer owns filesystem access, image decoding and encoding, metadata
handling, resize and compression operations, batch execution, cancellation, and
other operating-system integration.

The command boundary must use small, serializable request and response types.
Paths and user input must be validated in Rust before filesystem work begins.

## Development rules

- Support both macOS and Windows; avoid platform-specific assumptions in shared
  code.
- Prefer standard library and existing dependencies. Add a dependency only when
  its value, maintenance state, binary impact, and platform support are clear.
- Prefer direct, readable modules over speculative frameworks or abstractions.
- Keep filesystem and processing logic testable without the UI.
- Do not block the UI thread during native work.
- Processing errors should be isolated per file when possible so one bad input
  does not end an entire batch.
- Never silently overwrite a file; apply the user's selected conflict policy.
- Treat source files as read-only inputs unless overwrite is explicitly selected.
- Do not log image contents, full paths, or other private user data unnecessarily.
- Keep documentation aligned with implemented behavior.

## Versioning

- `package.json` is the canonical PixChisel version source.
- Versions follow semantic versioning: `MAJOR.MINOR.PATCH`.
- During pre-1.0 development, versions remain in `0.x.x`. Minor versions may
  identify meaningful development milestones; patch versions identify fixes and
  refinements.
- Version changes are intentional release preparation actions. Features and
  ordinary development builds must never bump the version automatically.
- Generated version fields in the Tauri configuration and Cargo manifest must be
  synchronized from `package.json` and checked before builds.
- Versioning scripts do not create Git tags or releases. Source-control and
  publishing actions remain explicit, separate release steps.

## Licensing and distribution

- PixChisel is free and open-source software under **GNU GPL v3.0 only**.
- Personal, educational, professional, and commercial use is permitted under
  the GPL.
- Modified distributions must provide corresponding source code under GPL v3.0.
- Public releases include source code, installers, the GPL license, and all
  required third-party notices and license texts.
- The PixChisel name and logo identify official releases. Modified products must
  not imply endorsement or present themselves as official PixChisel builds.
- Third-party code retains its original license. Never present a dependency's
  MIT, Apache, BSD, or other license as covering PixChisel itself.
- Final public-release terms and the complete target-specific dependency notice
  bundle must be reviewed before broad distribution. Project documentation
  records licensing intent and does not claim legal review.

## Branding assets

- `assets/branding/pixchisel-icon.png` is the canonical square application icon
  master for operating-system and installer surfaces.
- `assets/branding/pixchisel-logo.png` is the canonical horizontal logo master.
  It is not an application-icon source and should only be used where its wide
  composition, light lettering, and surrounding space are appropriate.
- Files under `src-tauri/icons/` are generated derivatives. Never edit them by
  hand or treat them as source artwork.
- Regenerate platform icon derivatives from the canonical icon master with
  `npm run tauri icon -- assets/branding/pixchisel-icon.png`.
- Keep both canonical masters unchanged unless replacement artwork is supplied
  and approved as an intentional branding change.

## Scope control

V1 includes JPEG, PNG, and WebP output. AVIF, HEIC/HEIF, TIFF, JPEG XL, crop,
watermarks, metadata inspection, presets, recursive folders, watch folders,
detail-enhancing upscaling, and comparison tools are deferred. Their future
status is not a reason to build extension systems during V1.

## Definition of done

A change is complete when it:

- respects the product and ownership boundaries above;
- handles relevant failure and cancellation paths;
- includes focused tests where logic warrants them;
- passes frontend and Rust checks;
- works with keyboard input and communicates state accessibly;
- has no unintended network dependency; and
- updates documentation when behavior or architecture changes.
