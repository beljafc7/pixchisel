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

1. The user adds one or more supported images with drag and drop or a file picker.
2. PixChisel displays each file's thumbnail, name, dimensions, format, and size.
3. The user chooses output format, compression, resize, metadata, destination,
   and naming conflict options.
4. The user starts the batch with a clearly labelled Chisel action.
5. PixChisel reports per-file and overall progress and allows cancellation.
6. PixChisel displays a result summary and makes the output location easy to open.

## Functional requirements

### Input and file list

- Accept multiple image files through drag and drop and a native file picker.
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

V1 accepts JPEG/JPG, PNG, and WebP input. The decoded file signature, rather than
the filename extension alone, determines whether an input is supported. Output
formats are fixed below.

### Output format

- Keep original format
- JPEG
- PNG
- WebP

Format-specific controls should appear only when meaningful. Transparent input
converted to JPEG uses a white background by default; the UI should communicate
that behavior where it affects the result.

Transformation settings are batch-level and session-only. The Phase 1.4 defaults
are keep original format, quality 82, no resize, no upscaling, and metadata
preservation. Settings are intentionally not persisted between launches.

### Compression

- Provide a quality control for lossy output formats.
- Present the control in understandable terms while keeping encoder-specific
  details out of the primary interface.
- Quality accepts whole numbers from 1 through 100 and is active for JPEG and
  WebP. It is disabled for PNG and keep-original output. JPEG and WebP map this
  value to their lossy encoders; exact sizes are intentionally not promised.

### Resize

- Resize to a target width or target height.
- Fit within a maximum width and maximum height.
- Resize by percentage.
- Always preserve aspect ratio in V1.
- Never produce zero or invalid dimensions.
- Do not upscale by default.
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
- Display progress for every file and for the batch overall.
- Allow cancellation and clearly distinguish completed, cancelled, skipped, and
  failed files.
- Continue past an individual failure where safe.
- Run at most three transformations concurrently while preserving queue order in
  the interface.
- Track ready, processing, written, skipped, failed, and cancelled states
  separately from import errors.
- Cancellation stops starting files; already-active files finish their safe
  native operation, and queued files become cancelled.
- Lock queue mutation, import, output, and transformation controls while a batch
  is active.
- Allow failed and cancelled items to be retried without reprocessing written or
  skipped items.

### Output

- Let the user select an output directory.
- Support overwrite, create-copy, and skip conflict behavior.
- Remove transferable image metadata from transformed output in the current V1
  pipeline and communicate that behavior directly.
- Avoid partially written final files by writing safely and finalizing only after
  successful encoding.
- Retain the selected directory and conflict policy only for the current session.
- Default conflict behavior to create-copy.

Output filenames retain the source stem. JPEG normalizes to `.jpg`, PNG to
`.png`, and WebP to `.webp`; keep-original uses the detected source format and
the same normalized extensions. Create-copy selects the first available numbered
sibling such as `photo (1).webp`. Skip is a successful non-write result.

Overwrite never encodes directly over an existing destination. The complete
encoded output is first written to a temporary file in the destination directory
and safely finalized. If source and destination resolve to the same filename,
the source remains intact until the replacement is complete and ready.

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

## Privacy and offline requirements

- No image or image-derived data leaves the device.
- No feature requires an internet connection after installation.
- No account or identity is required.
- No telemetry, analytics, crash upload, advertising, or tracking is enabled by
  default.
- Dependencies must not introduce hidden network behavior.

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
- Output follows the selected format, quality, resize, metadata, destination, and
  conflict settings.
- Progress, cancellation, errors, and results remain understandable throughout.
- Original files remain safe under every conflict policy.
- Packaged builds run on supported macOS and Windows versions.
