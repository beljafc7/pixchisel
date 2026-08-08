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

## Scope control

V1 includes JPEG, PNG, and WebP output. AVIF, HEIC/HEIF, TIFF, JPEG XL, crop,
watermarks, metadata inspection, presets, recursive folders, watch folders,
upscaling, and comparison tools are deferred. Their future status is not a reason
to build extension systems during V1.

## Definition of done

A change is complete when it:

- respects the product and ownership boundaries above;
- handles relevant failure and cancellation paths;
- includes focused tests where logic warrants them;
- passes frontend and Rust checks;
- works with keyboard input and communicates state accessibly;
- has no unintended network dependency; and
- updates documentation when behavior or architecture changes.
