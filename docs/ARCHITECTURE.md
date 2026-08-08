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

The application footer reads its version from Tauri's runtime app API. React does
not contain a hardcoded version or import package metadata into the frontend
bundle.

## Version metadata

`package.json` is the canonical authored version. The dependency-free
`scripts/version.mjs` utility copies that value into `tauri.conf.json` and the
Cargo package manifest only during an explicit version synchronization. Its
check mode performs no writes and fails when any of the three values differ.

Frontend and Tauri build scripts run the check mode before compiling. They never
silently synchronize or increment versions, so an accidental mismatch is visible
rather than modifying release metadata during normal development. Explicit npm
release scripts use `npm version --no-git-tag-version`; npm updates its package
metadata, then the version lifecycle synchronizes the native manifests.

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

The import command is `inspect_images(paths)`. It enters Tauri's blocking task
pool, uses at most four native inspection workers, and preserves input order in
its result array. Each path returns either image details or its own structured
error, so an invalid file cannot fail the rest of an import. The frontend invokes
one command per selection or drop instead of creating unbounded native calls.
Processing-job command and event payloads should be defined alongside the first
transformation spike rather than frozen prematurely.

React owns the transient queue. A queue item uses its native path as a stable ID,
and new inspection results are merged by exact path, allowing identical filenames
from different directories while preventing a repeated path. Ready items contain
format, dimensions, and size; error items retain the path and filename plus a
stable native error because valid image details are unavailable.

Desktop file drops are received through Tauri's webview drag/drop events, which
provide native filesystem paths. Browser drag/drop and HTML file inputs are not
used.

## Batch settings model

Phase 1.4 introduces one frontend-owned `BatchSettings` value for the complete
batch. It contains only JSON-serializable primitives: output format, quality, a
resize mode plus its numeric values, allow-upscaling, and remove-metadata. A pure
reducer owns every update, and pure validation derives field errors and request
readiness. UI controls do not maintain competing copies of these values.

The model retains values for inactive resize modes so a user can switch modes
without losing edits. Validation applies only to controls meaningful for the
selected output format and resize mode. Empty numeric controls are represented as
`NaN` transiently and fail validation; only valid settings may cross the future
native command boundary. Before processing is implemented, the Rust request type
must mirror this shape and independently enforce the same limits.

Settings live only for the mounted loaded-queue workspace. There is no browser
storage, database, or native preference persistence. The Chisel action remains
disabled until Phase 2 supplies a real native processing command.

## Transformation pipeline

Phase 2.1 mirrors the TypeScript settings contract in Rust under
`imaging/transform`. Serde uses camel-case naming and rejects unknown object
fields and enum values. Rust independently validates quality and the active
resize fields before opening or decoding a source. Encoded bytes remain in the
native `EncodedTransformation`; no Tauri command returns them and no destination
file is created.

The single-image pipeline detects JPEG, PNG, or WebP from content, obtains the
decoder orientation, decodes, applies orientation, calculates the resize target,
resamples only when needed, and encodes to the selected format. For
keep-original, the detected input format becomes the encoder format. This phase
always re-encodes. The current V1 metadata-removal policy means copying unchanged
source bytes would not be equivalent, so Phase 2.2 deliberately has no copy fast
path.

Resize calculation is pure. Width and height modes derive the other dimension;
fit chooses the limiting axis and never crops; percentage scales both axes.
Derived dimensions use integer half-up rounding and clamp positive results to at
least one pixel. When upscaling is disabled, any enlargement resolves to the
oriented source dimensions. Targets cannot exceed 32,768 pixels on either axis.
WebP has a stricter codec limit of 16,383 pixels per axis and returns a typed
format-specific error above it. Pixel resampling uses Lanczos3.

JPEG encoding uses the requested quality from 1 through 100. RGBA pixels are
explicitly composited over white before conversion to RGB. PNG encoding writes
RGBA and therefore retains pixel alpha. Lossy WebP uses the quality-aware
`webp` wrapper around statically built libwebp.

### Metadata reality

The `image` decoders can expose some EXIF, ICC, and XMP payloads, and selected
encoders can write subsets of them, but transferring them correctly across
orientation changes and format conversion is not uniform. PNG textual chunks
are also not retained by the current decode-to-pixels path. Phase 2.1 therefore
copies no source EXIF, ICC, XMP, or PNG textual metadata.

With `removeMetadata: true`, the result reports intentional removal. With it
false, the result reports `discardedUnsupported` rather than claiming
preservation. Encoder-required structural headers are not treated as source
metadata. A narrowly scoped preservation policy must be designed separately if
preservation is required before V1 release.

The V1 interface now presents this limitation as fixed behavior rather than an
editable option: transformed outputs remove transferable metadata. New frontend
settings therefore use `removeMetadata: true`; the field remains in the native
contract so behavior stays explicit.

## Output filesystem boundary

Phase 2.2 adds `write_transformed_image` for one image. Its request contains a
source path, user-selected output directory, validated batch settings, and one
of `overwrite`, `createCopy`, or `skip`. The command runs on Tauri's blocking
pool. React receives only paths and typed result metadata; encoded bytes never
cross the command boundary.

The output directory is chosen with the existing native dialog permission and
retained only in React session state. No general filesystem plugin permission is
granted. The native command validates that the directory still exists before
processing and performs every write itself.

Output names retain the source stem and replace only the extension: `.jpg` for
JPEG, `.png` for PNG, and `.webp` for WebP. Keep-original resolves the format from
file content and uses the normalized extension. Create-copy checks deterministic
`name (1).ext`, `name (2).ext` candidates up to 10,000. Final creation uses an
exclusive same-filesystem hard link from the complete temporary file, preventing
a race from overwriting a newly appeared destination.

Skip checks an existing deterministic destination before decoding or encoding.
Overwrite always transforms fully first, then writes, flushes, and syncs an
exclusive hidden temporary file in the destination directory. On Unix, rename
atomically replaces the destination. Standard Rust rename does not replace an
existing Windows file, so Windows first moves the completed destination to a
unique backup, moves the complete temporary file into place, restores the backup
if finalization fails, and removes it after success. This is recoverable but has
a brief non-atomic path transition on Windows.

When source and destination are the same, transformation completes in memory and
the temporary file is fully durable before replacement begins. Create-copy picks
a sibling, and skip leaves the source untouched. Failures before finalization do
not modify either source or an existing destination; temporary output is removed
on normal error paths where possible.

## Thumbnail pipeline

Ready imports request thumbnails through `generate_thumbnails(paths)`. The command
uses at most four workers and returns one ordered `ready` or `error` result per
source path. Metadata rows are committed before thumbnail generation begins, so
preview decoding does not block the queue from appearing. A thumbnail failure
changes only its preview state and never changes a valid import into an error.

Rust decodes the source, reads decoder orientation metadata, applies that
orientation, and then fits the complete image inside a 128 × 128 pixel box. It
does not crop or upscale. The existing `image` crate supports orientation for
JPEG and WebP, so no separate EXIF dependency is needed. PNG thumbnails are used
for every source because they render consistently and preserve transparent pixels.

Each application run creates a unique directory below
`$APPCACHE/thumbnails`. Source paths map to cached thumbnail entries for the
session, preventing regeneration while an item remains active. Removing an item
releases its cached file; Clear All releases all known files. The cache directory
is also removed when managed native state is dropped during a normal exit.
Thumbnails abandoned by an abnormal process termination may remain, but startup
does not read or depend on any previous session directory. A simple stale-session
prune can be added later if observed cache growth warrants it.

React receives only small cache paths and renders them with Tauri's asset
protocol. The protocol is enabled with the narrow static scope
`$APPCACHE/thumbnails/**/*`; source-image directories and the rest of the
filesystem are not exposed. React converts returned thumbnail paths with
`convertFileSrc` and does not retain decoded pixels or base64 image data.

## File safety

- Canonicalize or otherwise validate relevant paths in Rust.
- Never derive output paths by unvalidated string concatenation.
- Resolve conflicts immediately before the final write to reduce races.
- Write to a temporary file in the destination filesystem, then rename or replace
  according to the selected policy.
- Clean temporary files after failure or cancellation where possible.
- Do not modify source metadata or source files unless overwrite is selected.
- Never encode directly into a final path. Finalization occurs only after a
  complete same-directory temporary write has been flushed and synchronized.

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

The selected crate reads all V1 formats and encodes JPEG, PNG, and lossless WebP,
keeping inspection and most transformations in one ecosystem. Its pure-Rust
WebP encoder does not support lossy quality settings. Phase 2.1 therefore adds
the narrowly scoped `webp` safe wrapper for lossy WebP output. It is MIT or
Apache-2.0 licensed and uses `libwebp-sys` to statically compile the upstream
BSD-licensed libwebp sources. Users do not install a binary or shared library.
The build requires the normal C toolchain available to Rust desktop builds on
macOS and Windows, and increases compile time and binary size compared with the
pure-Rust lossless encoder.

Restricting features avoids the wider default format set, Rayon, AVIF tooling,
and unnecessary binary/dependency cost. The crate can expose some orientation,
EXIF, XMP, and ICC data depending on the decoder, but preservation and removal
behavior must be evaluated explicitly before metadata controls are built. No
secondary metadata library is required for thumbnail orientation.

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
