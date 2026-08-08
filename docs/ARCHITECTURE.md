# Architecture

## Overview

PixChisel is a two-layer local desktop application. React renders and coordinates
the experience; Rust performs trusted native and image work through Tauri
commands and events. There is no network service or persistence backend.

```text
React / TypeScript UI
  local view state, controls, validation, presentation
                 │
                 │ typed Tauri commands and events
                 ▼
Rust / Tauri core
  file inspection, decoding, processing, output, cancellation
                 │
                 ▼
Local filesystem only
```

## Frontend responsibilities

- Render the file queue, option controls, progress, and results.
- Hold transient UI and workflow state.
- Perform immediate presentation-level validation.
- Send serializable processing requests to Rust.
- Subscribe to typed progress events and display native errors safely.

The frontend must not read image bytes into JavaScript for processing. Thumbnail
delivery should use a Tauri-safe local mechanism selected during Phase 1, with
care taken not to retain large base64 payloads in React state.

### Initial source layout

```text
src/
  components/
    layout/       Shared application framing
  features/       Feature-owned UI and state (added as features land)
  lib/            Small frontend utilities and native command wrappers
  styles/         Global styles and future design tokens
  types/          Shared frontend domain types
  App.tsx         Top-level composition
  main.tsx        React entry point
```

Directories should be added when they have a real occupant. Feature code should
remain colocated rather than being split into global folders by file type.

## Native responsibilities

- Validate paths, processing options, and supported formats.
- Inspect file metadata and provide thumbnail-ready data.
- Decode, transform, encode, and safely write images.
- Schedule batch jobs outside the UI thread.
- Track cancellation and emit per-file and aggregate progress.
- Apply output naming and conflict rules.
- Return structured results and errors.

### Planned native layout

```text
src-tauri/src/
  commands/       Thin Tauri command adapters
  domain/         Processing requests, results, and domain rules
  imaging/        Decode, transform, metadata, and encode operations
  jobs/           Batch orchestration, progress, and cancellation
  filesystem/     Inspection, naming, and safe output writes
  lib.rs          Tauri application assembly
  main.rs         Desktop entry point
```

This is a target layout, not a requirement to create empty modules now. Commands
should delegate quickly to ordinary Rust functions so core behavior can be tested
without a running Tauri window.

## Command and event boundary

Requests and responses should use explicit serializable structures rather than
loosely typed maps. A batch request will eventually contain input paths, output
directory, format, quality, resize mode, metadata policy, and conflict policy.

Long-running processing should return a job identifier promptly. Progress and
completion should arrive as events that contain the job identifier and stable
file identifier. Cancellation should be cooperative: stop scheduling new work,
allow safe interruption points, clean temporary output, and report final states.

The first command is `inspect_image(path)`. It performs blocking file inspection
on Tauri's blocking task pool and returns filename, extension, detected format,
dimensions, and file size. Failures cross the boundary as a structured error with
a stable code and user-safe message. Batch command and event payloads should be
defined alongside the first processing spike rather than frozen prematurely.

## File safety

- Canonicalize or otherwise validate relevant paths in Rust.
- Never derive output paths by unvalidated string concatenation.
- Resolve conflicts immediately before the final write to reduce races.
- Write to a temporary file in the destination filesystem, then rename or replace
  according to the selected policy.
- Clean temporary files after failure or cancellation where possible.
- Do not modify source metadata or source files unless overwrite is selected.

## Concurrency and resource use

Image work is CPU- and memory-intensive. Batch concurrency must be bounded rather
than spawning one worker per input. The limit should account for available
parallelism and memory pressure. Progress events should be throttled enough to
avoid overwhelming the UI while remaining responsive.

## Dependency policy

PixChisel uses the pure-Rust `image` crate with default features disabled and only
the `jpeg`, `png`, and `webp` features enabled. It provides content-based format
detection and header-level dimension inspection without decoding the full pixel
buffer. It is mature, actively maintained, dual-licensed under MIT or Apache-2.0,
and introduces no native system library requirement on macOS or Windows.

The selected crate can read all V1 formats and encode JPEG, PNG, and lossless
WebP, keeping inspection and most future transformations in one ecosystem. Its
pure-Rust WebP encoder does not support lossy quality settings; Phase 2 must
evaluate whether lossless-only WebP satisfies the product or whether a narrowly
scoped encoder dependency is justified. This limitation is not a reason to add a
native system dependency during inspection work.

Restricting features avoids the wider default format set, Rayon, AVIF tooling,
and unnecessary binary/dependency cost. The crate can expose some orientation,
EXIF, XMP, and ICC data depending on the decoder, but preservation and removal
behavior must be evaluated explicitly before metadata controls are built. No
secondary metadata library is added in Phase 1.1.

The official Tauri dialog plugin supplies the native file picker. Only its open
permission is granted; save and message dialog permissions remain disabled.

Frontend dependencies should remain minimal. Tauri plugins are added only for
required native capabilities and granted the narrowest practical permissions.

## Security and privacy

- Content Security Policy should be made explicit before release; the starter
  currently uses a null CSP while the shell is local-only.
- Tauri capabilities should expose only commands and plugins the UI uses.
- Logs and errors should avoid full paths and user image content unless needed
  locally for diagnosis.
- The core workflow must make no network requests.

## Testing strategy

- Frontend: component tests for interaction and state transitions when the first
  interactive feature is introduced.
- Rust: unit tests for dimensions, names, conflicts, and option validation;
  integration tests with small fixture images for each supported format.
- End to end: focused happy-path, cancellation, corrupt-file, and conflict-policy
  flows on macOS and Windows.
- Packaging: verify signed/notarized distribution separately when release work
  begins.
