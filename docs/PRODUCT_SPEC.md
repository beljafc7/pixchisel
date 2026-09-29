# Product Specification

## Product summary

PixChisel is a free desktop utility that converts, compresses, and resizes image
files without sending them off-device. It serves people who want a quick,
trustworthy alternative to online image tools.

## Positioning

**Convert. Compress. Resize. Locally.**

The experience should be fast and linear: drop images, adjust options, process,
and finish. “Chisel” may be used as the primary action and in completion copy,
such as “Chisel 24 Images” and “Chiseled — 82% smaller.”

## Goals

- Make common image preparation tasks easy for non-specialists.
- Process multiple images efficiently in one operation.
- Make local-only behavior obvious and credible.
- Deliver a consistent macOS and Windows experience.
- Preserve a small, focused product surface.

## Non-goals

- Accounts, syncing, collaboration, or cloud storage
- Browser or server-based processing
- Professional image editing
- Automated asset pipelines or watched folders in V1
- Supporting every image format in V1

## V1 workflow

1. The user chooses Compress, Convert, or Resize.
2. The user adds supported images with drag and drop or a file picker.
3. PixChisel shows only settings relevant to that workflow, plus shared output controls.
4. The user starts the batch with a clearly labelled Chisel action.
5. PixChisel reports real per-image pipeline stages and allows cancellation.
6. PixChisel displays workflow-specific results and makes the output location easy to open.

Changing action is session-only and requires confirmation when it would clear a
loaded queue. All workflows adapt into the same native `BatchSettings` engine.

## Functional requirements

### Input and file list

- Accept multiple image files through drag and drop and a native file picker.
- Accept folders through drag and drop or a native directory picker, recursively
  discovering JPEG, PNG, and WebP candidates before content inspection.
- Recognize supported inputs and report unsupported or unreadable files clearly.
- Show thumbnail, filename, pixel dimensions, format, and file size.
- Allow files to be removed before processing.
- Prevent repeated native paths in the queue while allowing identical filenames
  from different directories.
- Preserve valid imports when another selected file is unsupported or corrupt,
  and show that failure on the affected queue row.
- Show a small, aspect-preserving thumbnail for each valid import after its
  metadata row appears. Thumbnail failure must not invalidate the imported image.
- Apply available orientation metadata to thumbnails, preserve transparency, and
  never crop or upscale thumbnail content.

V1 accepts JPEG/JPG, PNG, and WebP input. BMP remains unsupported. The decoded file signature, rather than
the filename extension alone, determines whether an input is supported. Output
formats are fixed below.

Folder discovery is deterministic, does not follow directory symlinks, and
ignores dot-hidden entries. Unsupported files found inside folders are silently
omitted; explicitly supplied unsupported files retain actionable errors.
Overlapping folders and direct-file duplicates resolve to one path-based item.

### Convert

Convert exposes JPEG, PNG, and WebP targets only. It applies no resize. JPEG
uses internal quality 92, PNG is lossless, and WebP uses the benchmarked internal
quality 75. The WebP value reduced a representative seventeen-image batch from
29.35 MB at quality 92 to 11.13 MB while remaining visually comparable to the
10.03 MB reference output. Numeric quality remains an internal implementation
detail. Transparent pixels converted to JPEG use a white background.

Transformation settings are batch-level and session-only. The Phase 1.4 defaults
are keep original format, quality 82, no resize, no upscaling, and metadata
preservation. Settings are intentionally not persisted between launches.

### Compress

Compress preserves source format and offers Standard, Strong, and Maximum.
JPEG and WebP map these to internal qualities 82, 65, and 45. PNG remains
lossless and maps the same choices to Oxipng effort presets 2, 4, and 6. These
PNG choices affect optimization effort, never pixel values or color count. A Phase 2.7 JPEG
benchmark retained these values: lowering Maximum further gave limited savings
on smooth and graphic material while quality metrics and sharp-detail inspection
continued to deteriorate. JPEG output uses the pure-Rust `jpeg-encoder` with
4:4:4 sampling, progressive scans, and optimized Huffman tables after it measured
about 3–23% smaller than the prior image-rs encoder at comparable PSNR on the
generated benchmark corpus. Keep-original means original format, not original
bytes.

Compress has a native never-grow guarantee for JPEG, PNG, and WebP. PixChisel
encodes and optimizes in memory first. If the candidate is not strictly smaller
than the source, it does not create or replace a file and reports Already
Optimized. Convert and Resize may legitimately produce larger output and are not
subject to this rule.

### Resize

- Resize to a target width or target height.
- Fit within a maximum width and maximum height.
- Resize by percentage.
- Always preserve aspect ratio in V1.
- Never produce zero or invalid dimensions.
- Do not upscale by default.
- Preserve input format and use internal quality 92 for JPEG and WebP. PNG
  remains lossless.
- Dimension values accept whole numbers from 1 through 32,768 pixels. Percentage
  accepts whole numbers from 1 through 1,000. These conservative UI limits keep
  future native requests bounded; Rust must validate them again before processing.
- Aspect-derived dimensions use half-up rounding and never become smaller than
  one pixel. WebP output has a codec-specific maximum of 16,383 pixels per axis.

### Transformation behavior

- Orientation is applied before resize calculations and pixel resampling.
- Keep original means re-encoding to the detected source format. The current
  metadata-removal policy prevents an unchanged-byte copy optimization.
- JPEG output composites transparent pixels onto white.
- PNG output preserves pixel alpha.
- Source files remain untouched during transformation.

The current re-encode pipeline does not preserve source EXIF, ICC, XMP, or PNG
textual metadata. The V1 interface truthfully states that transformed outputs
remove metadata rather than offering a preservation control the encoder pipeline
cannot honor.

### Batch processing

- Apply configured options to all files in the batch.
- Display stage progress for every file and a compact batch total.
- Allow cancellation and clearly distinguish completed, cancelled, skipped, and
  failed files.
- Continue past an individual failure where safe.
- Run at most two transformations concurrently while preserving queue order in
  the interface.
- Track ready, processing, written, skipped, failed, and cancelled states
  separately from import errors.
- Cancellation stops starting files and signals active native transformations.
  Active work stops at the next safe checkpoint; WebP encoding can stop through
  libwebp's progress hook. Cancelled work never finalizes an output, and queued
  files become cancelled.
- Cancellation updates every unfinished queue row immediately. If a codec cannot
  interrupt an operation already in progress, the interface reports that safe
  cleanup is finishing instead of leaving one row indefinitely in a cancelling
  state; processing controls unlock after that native work exits without saving.
- Lock queue mutation, import, output, and transformation controls while a batch
  is active.
- Allow failed and cancelled items to be retried without reprocessing written or
  skipped items.

Per-image milestones are Preparing 5%, Decoding 20%, Optimizing 45%, Encoding
70%, Saving 90%, and Completed 100%. They are real pipeline boundaries, not ETA
or time estimates. Skipped files never report false completion, and work not
started after cancellation becomes Cancelled.

### Output

- Expose two save modes: Create Copies and Replace Originals.
- Create Copies leaves every source untouched. When all ready images share one
  parent directory, it defaults to a `PixChisel Copies` subfolder there and lets
  the user choose a different destination. Mixed-directory batches require an
  explicitly selected output folder. Copies are flattened into the resolved
  destination and use the first available numbered filename when needed.
- Replace Originals requires no output folder. Every item uses its own source
  directory, including mixed-directory and recursively imported batches.
- Display a concise warning that replacing originals cannot be undone.
- Remove transferable image metadata from transformed output in the current V1
  pipeline and communicate that behavior directly.
- Avoid partially written final files by writing safely and finalizing only after
  successful encoding.
- Retain the selected directory and save mode only for the current session.
- Default to Create Copies.
- Verify a selected copy destination, or the parent of an automatic destination,
  before starting a Create Copies batch.
- Create the automatic subfolder only when a transformed output will be written.
- Offer an Open Output Folder action after the resolved destination exists.

Copy filenames retain the source stem. JPEG normalizes to `.jpg`, PNG to `.png`,
and WebP to `.webp`; keep-original uses the detected source format. Create Copies
selects the first available numbered sibling such as `photo (1).webp`. Recursive
folder discovery skips PixChisel's generated `PixChisel Copies` directories so
later imports do not pull generated outputs back into the source queue.

Replace Originals never encodes directly over an existing destination. The complete
encoded output is first written to a temporary file in the destination directory
and safely finalized. If source and destination resolve to the same filename,
the source remains intact until the replacement is complete and ready.

Convert plus Replace Originals writes the converted extension beside the source
and removes the source only after the new file has been finalized. If that target
already belongs to another file, the item fails with a conflict; PixChisel does
not overwrite it or invent a numbered replacement. Compress and Resize preserve
the detected format and replace the exact source path.

### Results

Show:

- original total size;
- final total size;
- absolute amount saved; and
- percentage saved.

The summary must handle outputs larger than inputs without misleading language.
Only written files contribute to byte totals. Skipped, failed, and cancelled
counts remain visible independently. A zero-byte original total must never cause
division by zero.

Results describe one configuration snapshot. Changing any transformation or
output setting clears the previous row results and summary. Removing an item
removes its result from the summary. Clear All removes queue-specific results,
errors, and thumbnail references while retaining session settings and the output
folder.

## Privacy and offline requirements

- No image or image-derived data leaves the device.
- No feature requires an internet connection after installation.
- No account or identity is required.
- No telemetry, analytics, crash upload, advertising, or tracking is enabled by
  default.
- Dependencies must not introduce hidden network behavior.

## Licensing model

PixChisel is proprietary freeware. People and businesses may download and use
the application free of charge for personal, educational, professional, and
commercial work, including processing images for clients or internal business
use. PixChisel is not open source. The application license does not grant rights
to reuse source code, redistribute modified builds, resell or rebrand the
software, create derivative distributions, or use PixChisel branding.

V1 installers are distributed only through official PixChisel sources. Public
packages contain compiled application binaries, the PixChisel license, and all
required third-party notices and license texts; they do not contain PixChisel
source code.

## Experience requirements

- Use one primary workspace rather than dashboard navigation.
- Keep advanced choices secondary to the main flow.
- Provide keyboard-accessible controls, visible focus, semantic status updates,
  and sufficient contrast.
- Use native dialogs for file and directory selection.
- Follow the operating system's light/dark preference where practical.

## Deferred features

AVIF, HEIC/HEIF, TIFF, JPEG XL, crop, watermark, metadata inspector, presets,
recursive folder processing, watch folders, upscaling, and image comparison tools
are explicitly outside V1.

## V1 success criteria

- A user can process a mixed multi-file batch without network access.
- Output follows the selected format, quality, resize, metadata, and save settings.
- Progress, cancellation, errors, and results remain understandable throughout.
- Originals remain safe when creating copies and until replace-original output
  has been fully transformed, written, synchronized, and finalized.
- Packaged builds run on supported macOS and Windows versions.

## Alpha readiness

PixChisel Alpha is ready when the macOS end-to-end workflow has been manually
verified, no known data-loss defect remains, automated tests and packaged builds
pass, branding and version metadata are correct, output safety is validated, and
major workflow blockers are closed. Windows must either pass its platform QA
checklist or be explicitly labelled not yet tested for that alpha.
