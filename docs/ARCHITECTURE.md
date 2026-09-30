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

## Workflow adapters

`WorkflowMode` is session-level frontend state with `compress`, `convert`, and
`resize` variants. Workflow selection precedes import. The three small adapters
produce the existing native `BatchSettings`, so decoding, transformation,
encoding, safe writing, cancellation, and results remain one shared engine.

Compress preserves format and offers two explicit tradeoffs. Standard uses
JPEG/WebP quality 82 and lossless PNG optimization. Maximum uses JPEG/WebP
quality 45 and RGBA-aware palette quantization for PNG. Convert targets a
selected format with resize disabled: JPEG and PNG use the existing internal
value 92, while WebP uses the corpus-selected value 75. Resize preserves format
at internal quality 92.

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

## Folder discovery

React sends selected or dropped paths to the narrow `discover_images` command.
Rust keeps explicit files in input order and recursively expands directories on
the blocking task pool. Folder candidates use case-insensitive JPEG, PNG, and
WebP extensions as an initial filter, are sorted by full path, and then enter the
existing content-based inspector.

Traversal skips symbolic links to prevent cycles and ignores dot-hidden entries.
Unsupported folder contents are silent, while explicitly supplied unsupported
files remain actionable. Generated directories named `PixChisel Copies` are
also skipped so a later recursive import does not re-ingest automatic outputs.
Results are deduplicated by path and append without reordering the existing
queue. Output selection remains separate; V1 flattens folder batches into the
resolved destination rather than preserving hierarchy.
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
loosely typed maps. Native write requests contain a source path, validated batch
settings, and a discriminated save destination rather than ambiguous output
directory and conflict fields.

Long-running processing should return a job identifier promptly. Progress and
completion should arrive as events that contain the job identifier and stable
file identifier. Cancellation should be cooperative: stop scheduling new work,
allow safe interruption points, clean temporary output, and report final states.
The interface marks unfinished rows cancelled immediately. A codec operation
without an interruption hook may continue draining on its blocking worker, but
the UI reports that safe cleanup explicitly and no cancelled output is finalized.

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
resize mode plus its numeric values, derived allow-upscaling, and remove-metadata. A pure
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
Width, height, and percentage enable upscaling implicitly so their requested
target is honored. Fit disables upscaling because its dimensions are maximum
bounds. Derived dimensions use integer half-up rounding and clamp positive
results to at least one pixel. Targets cannot exceed 32,768 pixels on either axis.
WebP has a stricter codec limit of 16,383 pixels per axis and returns a typed
format-specific error above it. Pixel resampling uses Lanczos3.

JPEG encoding uses `jpeg-encoder` 0.7.1 with quality from 1 through 100. RGBA pixels are
explicitly composited over white before conversion to RGB. Standard PNG first
writes lossless RGBA with image-rs and then runs Oxipng 10.2.0 preset 2. It
preserves every decoded RGBA value, including hidden RGB behind transparent
pixels. Maximum PNG uses the MIT-licensed `color_quant` 2.0.0 NeuQuant
implementation to produce an indexed palette of at most 256 RGBA colors, then
runs the same bounded Oxipng preset 2. Maximum therefore preserves dimensions
and transparency support but may alter colors and alpha levels. Source metadata
is omitted by the decode-to-pixels pipeline. Lossy WebP uses the quality-aware
`webp` wrapper around statically built libwebp.

The earlier lossless-only PNG implementation mapped Standard, Strong, and
Maximum to Oxipng presets 2, 4, and 6. Real 3K–6K images showed that Strong took
about 12 minutes and Maximum about 24 minutes for only 6–7% total improvement
over Standard. The effort-only tiers were removed because they were not useful
product choices. `libimagequant` was also rejected: its free license is
GPLv3-or-later and proprietary distribution requires a commercial license.

The replacement Maximum benchmark processed eleven valid real-world images in
about 83 seconds in a release build. Applying the native never-grow rule reduced
the comparable batch from 81,013,992 bytes to 21,972,792 bytes. The seven PNGs
measured approximately 43.69–48.00 dB PSNR where changed; one already-efficient
PNG remained byte-for-byte unchanged. These measurements establish a regression
baseline, not a promise that every corpus will reach the same ratio.

The selected encoder uses 4:4:4 sampling to protect color detail, progressive
output, and image-specific optimized Huffman tables. The previous image-rs
0.25.10 encoder used baseline sequential output and fixed standard Huffman
tables. Although image-rs documented 4:2:2, its implementation used equal
sampling factors and was effectively 4:4:4 for PixChisel's RGB input. PixChisel
does not copy JPEG ICC or EXIF data.

Phase 2.7 benchmarked qualities 95, 90, 85, 82, 75, 65, 55, 45, 35, and 25 on
generated photographic-style, high-detail, gradient, flat-color, and sharp-edge
images. All 50 outputs decoded at 1024×768. PSNR was supporting evidence, not a
substitute for perception. At qualities 82/65/45, results were: photographic
39,385/31,678/27,988 bytes at 49.26/46.91/44.40 dB; high-detail
1,133,247/813,353/606,526 bytes at 25.00/19.60/15.98 dB; and sharp-edge
102,898/87,401/78,958 bytes at 44.59/39.51/38.44 dB. Release-mode encoding took
approximately 5–19 ms per image at those presets on the benchmark Mac. Moving
from quality 45 to 35 saved only about 1–7% on four non-noise cases while
degradation continued, so the retained Standard and Maximum values are 82 and
45.

The same corpus then compared pure-Rust `jpeg-encoder` with progressive output,
optimized Huffman tables, and both 4:2:0 and 4:4:4 sampling. 4:2:0 was rejected:
it reduced bytes sharply but caused severe PSNR loss on chroma-heavy detail and
flat-color edges. At 4:4:4, the alternative retained essentially equivalent PSNR
while producing files about 3–23% smaller across the corpus and presets. It was
therefore selected. It adds one approximately 150 KB source crate and no native
library, external executable, build tool, or runtime dependency; final packaged
binary delta still requires release-build measurement. The comparable macOS
debug executable increased by 464,208 bytes (about 1.2%), from 38,729,640 to
39,193,848 bytes.

MozJPEG was not added because its trellis and scan optimization require a
C/assembly build and native-library distribution. Jpegli offers further
perceptual techniques but adds a C++ build and less established Rust integration.
The selected pure-Rust crate is cross-platform, supports macOS and Windows, and
is licensed under `(MIT OR Apache-2.0) AND IJG`.

### Metadata reality

The `image` decoders can expose some EXIF, ICC, and XMP payloads, and selected
encoders can write subsets of them, but transferring them correctly across
orientation changes and format conversion is not uniform. PNG textual chunks
are also not retained by the current decode-to-pixels path. Phase 2.1 therefore
copies no source EXIF, ICC, XMP, or PNG textual metadata.

Resize is the narrow exception for physical resolution. Before pixel decoding,
the native pipeline reads JPEG JFIF density or the PNG `pHYs` chunk and passes
only that value to the matching encoder. This keeps a 300 DPI source at 300 DPI
after resizing without retaining descriptive, location, camera, or timestamp
metadata. WebP has no native physical-resolution field outside metadata
containers, so the current policy does not synthesize one.

With `removeMetadata: true`, the result reports intentional removal. With it
false, the result reports `discardedUnsupported` rather than claiming
preservation. Encoder-required structural headers and Resize's narrowly
preserved physical resolution are not treated as transferable source metadata.
Any broader preservation policy must be designed separately.

The V1 interface now presents this limitation as fixed behavior rather than an
editable option: transformed outputs remove transferable metadata while Resize
retains supported physical-resolution values. New frontend settings therefore
use `removeMetadata: true`; the field remains in the native contract so behavior
stays explicit.

## Output filesystem boundary

`write_transformed_image` receives a source path, validated batch settings, and
either `{ mode: "directory", path }` or `{ mode: "replaceOriginal" }`. Invalid
combinations such as replace-original plus an arbitrary output folder cannot be
represented. The command runs on Tauri's blocking pool. Encoded bytes never
cross the command boundary.

For Compress only, the writer compares encoded candidate length with the source
length before emitting the Saving stage or touching the destination. A candidate
that is equal or larger returns the typed `notSmaller` result. React renders it
as Already Optimized and excludes it from written-byte savings totals. This
applies uniformly to JPEG, PNG, and WebP; Convert and Resize remain allowed to
grow.

Create Copies resolves automatically to one `PixChisel Copies` subfolder when
all ready sources share a parent directory. React derives that path without
touching the filesystem; a native write creates it only after a transformed
candidate is ready and eligible to be saved. The user can override it with the
native directory dialog. Mixed-directory batches require that explicit shared
destination, preventing outputs from being scattered across source folders.
Selecting Create Copies again clears the override and restores the automatic
destination when available. No general filesystem or shell plugin permission is
granted. Before a batch starts, a narrow native preflight verifies the resolved
destination or its parent exists and accepts an exclusive-create probe file.
Every individual write validates or creates the resolved directory again.
Free space is deliberately not predicted because that check would become stale
during processing.

Opening the destination is also a narrow native command. It validates the saved
directory and invokes Finder on macOS or Explorer on Windows; React cannot pass
an executable, arguments, or arbitrary shell command.

Output names retain the source stem and replace only the extension: `.jpg` for
JPEG, `.png` for PNG, and `.webp` for WebP. Keep-original resolves the format from
file content and uses the normalized extension. Create-copy checks deterministic
`name (1).ext`, `name (2).ext` candidates up to 10,000. Final creation uses an
exclusive same-filesystem hard link from the complete temporary file, preventing
a race from overwriting a newly appeared destination. If the destination
filesystem does not support hard links, PixChisel falls back to opening the final
path with exclusive creation and copying the already-complete temporary file.
The fallback never opens an existing destination, synchronizes the new file, and
removes a partial destination if copying fails. Hard links remain preferable
because their finalization is atomic; the portable fallback can briefly expose a
partial new file to other processes, but never overwrites user data.

Replace Originals resolves each source's parent folder independently. Same-format
Compress and Resize always transform fully first, then write, flush, and sync an
exclusive hidden temporary file in the destination directory. On Unix, rename
atomically replaces the destination. Standard Rust rename does not replace an
existing Windows file, so Windows first moves the completed destination to a
unique backup, moves the complete temporary file into place, restores the backup
if finalization fails, and removes it after success. This is recoverable but has
a brief non-atomic path transition on Windows.

When source and destination are the same, transformation completes in memory and
the temporary file is durable before replacement begins. Format conversion
finalizes the new extension without overwrite and removes the source only after
that succeeds. A pre-existing converted target is a conflict and remains
untouched. Failures before finalization do not modify the source or an existing
destination; temporary output is removed on normal error paths where possible.

## Batch orchestration

Phase 2.3 keeps orchestration state in React and reuses the existing
`write_transformed_image` command for every ready queue item. A small worker pool
starts at most two native writes concurrently. Rust remains responsible for
each transformation and safe filesystem finalization; encoded bytes never enter
React. Import-error rows are excluded before a batch starts, and result updates
are keyed by source path so the visible queue remains in import order even when
native calls finish out of order.

The limit was reduced from three after a debug-build QA run with three concurrent
24-megapixel JPEG transformations reached roughly 555 MiB resident memory. Two
workers preserve parallel progress while reducing peak pressure on lower-memory
systems. The limit should be revisited only with platform measurements.

Per-file processing state is separate from import validity: `ready`,
`processing`, `written`, `skipped`, `failed`, or `cancelled`. One failed native
call is recorded on its own item and does not stop another worker. Disk-full and
other write failures use the same isolation; PixChisel currently does not
pre-estimate free space.

Cancellation uses a batch-scoped atomic signal shared between React and active
Rust commands. The frontend token immediately prevents workers from claiming
another queued item. Native transforms check the same signal between decoding,
resizing, encoding, and safe-write boundaries. WebP additionally installs
libwebp's progress callback so a long encode can abort from inside the codec.
Cancelled work never finalizes an output; temporary files are removed by their
owner on exit. A decoder or non-WebP codec call that does not expose an interrupt
hook may still need to return to the next checkpoint before cancellation settles.

Progress uses real pipeline milestones rather than invented byte-level values. A
typed Tauri channel sends source path, stage, and fixed percentage at Preparing
5, Decoding 20, Optimizing 45, Encoding 70, Saving 90, and Completed 100. These
values are execution boundaries, not ETA estimates. Overall progress counts
terminal items against the batch total. Result aggregation counts written,
skipped, failed, and cancelled items
and sums original/output bytes only for written results. A zero-byte original
total produces a zero percentage, while larger output is described as larger
rather than negative savings.

Completed states remain visible until the queue or batch configuration changes.
Changing format, quality, resize, output directory, or save mode
clears all prior row results and the summary so they cannot appear to describe
the new configuration. Removing one item removes only that item's result and
recalculates the summary. A new full Chisel action resets processing state and
processes every current ready item. Retry Failed & Cancelled resets and processes
only those items, leaving written and skipped results untouched.

Clear All removes the queue, import errors, thumbnail references, processing
states, and summary. The mounted options state deliberately retains batch
settings, save mode, and any selected copy folder for convenient repeated work. Queue
mutation and all processing settings are locked only while a batch is active.

Changing workflow requires confirmation when a queue is loaded, then clears that
workspace rather than attempting cross-workflow migration.

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
Thumbnail sessions abandoned by an abnormal process termination are pruned on a
later startup once they are more than 24 hours old. Recent session directories
are retained so launching another instance cannot disrupt its active previews.
Startup never reads or depends on thumbnail contents from a previous session.

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
- Do not modify source metadata or source files unless Replace Originals is selected.
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

The selected crate reads all V1 formats and encodes PNG and lossless WebP,
keeping inspection and most transformations in one ecosystem. `jpeg-encoder`
provides the evidence-selected progressive, optimized-Huffman JPEG output. The image crate's pure-Rust
WebP encoder does not support lossy quality settings. Phase 2.1 therefore adds
`libwebp-sys` statically compiles the upstream BSD-licensed libwebp sources for
lossy WebP output. PixChisel uses the native picture API directly so it can
install libwebp's cancellation progress hook. Users do not install a binary or shared library.
WebP uses libwebp method 2. A release benchmark on seventeen 15–24 megapixel
JPEGs at quality 92 measured 6.7 seconds and 29.35 MB, compared with 13.1
seconds and 28.17 MB for method 4. On ten matched images, average PSNR changed
from 46.90 to 46.79 dB. The approximately 2× speed improvement was selected in
exchange for 4.2% more bytes and a negligible measured fidelity difference.
The dev profile optimizes only `libwebp-sys`; this reduced the same debug corpus
from an extrapolated multi-minute run to 40.9 seconds without changing release
optimization or application quality settings.

A follow-up quality matrix measured WebP qualities 92/85/82/78/75/70 on the
same seventeen JPEGs. Quality 75 produced 11.13 MB in 5.14 seconds at average
43.41 dB PSNR and 0.9764 SSIM. The external comparison output was 10.03 MB at
43.48 dB and 0.9773 SSIM. Quality 75 was therefore selected for Convert-to-WebP:
it stays visually and objectively comparable while avoiding quality 70's lower
fidelity and quality 78's 12.54 MB size. JPEG conversion and Resize remain at
quality 92.
The build requires the normal C toolchain available to Rust desktop builds on
macOS and Windows, and increases compile time and binary size compared with the
pure-Rust lossless encoder.

Oxipng 10.2.0 supplies the PNG optimization pass under the MIT license. With its
default features disabled it still uses the Apache-2.0 `libdeflater` and
`libdeflate-sys` crates, which statically build libdeflate and add no runtime
executable or shared-library requirement. This adds native C compilation to the
PNG path's build graph; PixChisel already requires a native toolchain for the
statically built WebP codec.

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

## Release and license boundary

PixChisel source code and documentation are released under GNU GPL v3.0 only.
Third-party dependencies remain under their respective licenses. The PixChisel
name and logo identify the official project; GPL does not grant permission to
imply endorsement or present a modified product as an official release.

Official macOS and Windows installers must be accompanied by equivalent access
to the corresponding source, the GPL license, and a target-specific third-party
notice bundle with all required texts and attributions. Release preparation must
derive the dependency inventory from the locked production graph for each target,
including statically linked libwebp, rather than treating a hand-maintained
summary as exhaustive.

## Testing strategy

- Frontend: component tests for interaction and state transitions when the first
  interactive feature is introduced.
- Rust: unit tests for dimensions, names, conflicts, and option validation;
  integration tests with small fixture images for each supported format.
- End to end: focused happy-path, cancellation, corrupt-file, and conflict-policy
  flows on macOS and Windows.
- Packaging: verify signed/notarized distribution separately when release work
  begins.
