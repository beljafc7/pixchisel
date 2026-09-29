# Roadmap

The phases below are ordered to validate the riskiest native behavior early while
keeping each increment usable and reviewable. Scope may be adjusted, but V1 and
future features should not be mixed silently.

## Foundation — complete

- Remove starter demo behavior and branding.
- Establish product, architecture, and engineering rules.
- Create the minimal PixChisel application shell.
- Confirm frontend and Rust builds.
- Establish proprietary-freeware licensing intent, private-source policy, and
  initial third-party notice requirements. **Complete in Phase 2.6.1.**

## Phase 1 — Import and inspection

- Decide and document the exact V1 input format matrix. **Complete:** JPEG, PNG,
  and WebP.
- Evaluate and select the Rust imaging dependency. **Complete:** `image` with
  restricted format features.
- Add single-file native selection and typed inspection. **Complete in Phase
  1.1.**
- Expand selection to the multi-file picker and add drag-and-drop ingestion.
  **Complete in Phase 1.2.**
- Inspect supported files in bounded native batches. **Complete in Phase 1.2.**
- Display thumbnail, filename, dimensions, format, and file size. **Complete in
  Phase 1.3.**
- Handle duplicates, unsupported formats, and corrupt images. **Complete in Phase
  1.2.**
- Establish typed frontend/native request and error contracts. **Complete for
  import inspection.**
- Add the batch transformation-options UI and typed, validated frontend settings
  model. **Complete in Phase 1.4; no processing is performed.**

Exit condition: users can build and edit a multi-image queue; no processing is
performed.

## Phase 2 — Single-image processing

- Mirror and validate `BatchSettings` in Rust. **Complete in Phase 2.1.**
- Implement JPEG, PNG, WebP, and keep-original encoding using the selected
  output setting; decide the lossy WebP encoder strategy first. **Complete
  in-memory in Phase 2.1 using statically built libwebp for lossy WebP.**
- Implement quality and aspect-preserving resize behavior. **Complete in-memory
  in Phase 2.1.**
- Apply the documented white JPEG transparency background and no-upscale policy.
  **Complete in the native transformation pipeline.**
- Add output directory selection and the three conflict policies. **Complete for
  the session-scoped single-image boundary in Phase 2.2.**
- Implement safe native output writes. **Complete for single-image output in
  Phase 2.2, including source-equals-destination protection.**
- Add metadata removal where supported. **Complete for the current V1 policy:
  transformed outputs explicitly remove transferable metadata while Resize
  retains JPEG and PNG physical resolution.**

### Phase 2.2 — Filesystem boundary — complete

- Select an output directory through a native dialog. **Complete.**
- Define the native single-image write request around the established settings.
  **Complete.**
- Implement overwrite, create-copy, and skip behavior without silent data loss.
  **Complete.**
- Write encoded bytes to a temporary sibling and finalize atomically where the
  platform and destination filesystem permit. **Complete.**
- Surface the metadata-preservation limitation before enabling production output.
  **Complete: V1 transformed outputs explicitly remove metadata while Resize
  retains supported physical resolution.**

### Phase 2.3 — Batch boundary — complete

- Connect the existing Chisel action to bounded native batch orchestration.
  **Complete with two concurrent native writes after Phase 2.4 memory QA.**
- Reuse the single-image writer for each ready queue item. **Complete.**
- Add stable job and per-file state without returning encoded bytes to React.
  **Complete.**
- Add progress and cooperative cancellation without weakening temporary-file
  cleanup or conflict guarantees. **Complete at safe file boundaries.**
- Keep individual write failures isolated and report skipped files separately.
  **Complete.**

### Phase 2.4 — Workflow refinement

- Add destination preflight and actionable write-failure messages. **Complete.**
- Add a native Open Output Folder affordance without exposing shell access.
  **Complete.**
- Clear completed results when settings change; retain settings and destination
  across Clear All. **Complete.**
- Add a safe create-copy fallback for filesystems without hard links.
  **Complete; external-volume runtime QA remains.**
- Review focus, screen-reader announcements, compact layout, and repeated runs.
  **Complete for the implemented UI; formal assistive-technology QA remains.**
- Perform focused macOS and Windows end-to-end QA with representative large
  batches and destination filesystems. **macOS partially complete; Windows and
  removable-drive testing remain.**

### Phase 2.5 — Workflow architecture

- Add session-level Compress, Convert, and Resize selection before import.
  **Complete.**
- Adapt workflow controls into the shared native settings engine. **Complete.**
- Replace numeric compression quality with Standard and Maximum.
  **Complete.**
- Fix original-format JPEG/WebP quality resolution. **Complete.**
- Add typed native per-image stage milestones. **Complete.**
- Revalidate create-copy, overwrite, and skip across serialized workflow
  settings. **Automated coverage complete; desktop click-through QA remains.**

### Phase 2.6 — Folder import and conflict simplification

- Add native recursive discovery for selected and dropped folders. **Complete.**
- Reuse bounded inspection, thumbnails, path deduplication, and queue ordering.
  **Complete.**
- Skip directory symlinks and silently ignore unsupported folder contents.
  **Complete.**
- Expose only Create Copy and Replace Existing in V1. **Completed in Phase 2.6,
  then superseded by destination-oriented save modes in Phase 2.7.**
- Keep output selection explicit and flatten folder batches into the selected
  destination. **Complete.**

### Phase 2.7 — JPEG evidence and save destinations

- Benchmark image-rs JPEG output across representative generated content,
  quality points, decode checks, timing, and PSNR. **Complete.**
- Review specialized encoders without adding an unproven production dependency.
  **Complete; pure-Rust `jpeg-encoder` selected, 65 evaluated, and product
  presets retained at 82/45.**
- Replace conflict-oriented UI state with Create Copies and Replace Originals.
  **Complete.**
- Require a folder only for Create Copies and replace mixed-directory sources in
  their own locations. **Complete.**
- Default same-directory Create Copies batches to a lazily created `PixChisel
  Copies` subfolder while retaining manual selection for mixed sources.
  **Complete.**
- Make converted-target conflicts non-destructive and remove the source only
  after successful converted-file finalization. **Complete.**

### Phase 2.7.1 — PNG optimization and never-grow compression

- Replace the fast default PNG output with in-memory Oxipng optimization using
  a bounded Standard effort preset. **Complete.**
- Preserve exact RGBA pixels and transparency in Standard. **Complete.**
- Replace the unproductive Strong and lossless Maximum effort tiers with one
  perceptual Maximum option using an RGBA-aware indexed palette. **Complete.**
- Prevent JPEG, PNG, and WebP Compress operations from writing candidates that
  are equal to or larger than their sources. **Complete with typed Already
  Optimized results.**
- Validate representative real-world PNGs on both release platforms. **Automated
  generated-corpus coverage is complete; macOS real-file and Windows package QA
  remain.**

### Phase 2.7.2 — Active cancellation

- Send a batch-scoped cancellation signal from React to active native jobs.
  **Complete.**
- Stop transforms at safe native checkpoints and prevent cancelled work from
  finalizing an output. **Complete.**
- Interrupt long WebP encoding through libwebp's progress callback. **Complete.**
- Measure cancellation latency on representative large JPEG-to-WebP batches in
  release builds. **Implementation complete; desktop timing QA remains.**

### Phase 2.7.3 — WebP conversion performance

- Benchmark the seventeen-image large JPEG corpus in debug and release builds.
  **Complete.**
- Optimize libwebp in development builds without changing output quality.
  **Complete.**
- Select a faster evidence-backed libwebp effort method. **Complete; method 2
  halves release time with a 4.2% size cost and negligible measured PSNR change.**
- Keep two concurrent full-image jobs until peak-memory profiling justifies a
  higher cross-platform limit. **Retained for safety.**
- Calibrate Convert-to-WebP output weight against the same corpus and visual
  reference. **Complete; internal quality 75 selected at 11.13 MB, 43.41 dB
  PSNR, and 0.9764 SSIM.**

Exit condition: one image can be transformed locally with predictable output and
source-file safety.

## Phase 3 — Batch jobs — completed early in Phase 2.3

- Introduce bounded native batch execution. **Complete.**
- Add per-file and overall progress. **Complete.**
- Add cooperative cancellation and temporary-file cleanup. **Complete.**
- Isolate individual file failures. **Complete.**
- Keep the UI responsive for large queues. **Complete by using asynchronous
  native calls with a two-item worker pool.**

Exit condition: a batch can complete or cancel cleanly with an accurate state for
every file.

## Phase 4 — Results and refinement

- Add original size, final size, amount saved, and percentage saved.
- Handle size increases and partial failures clearly.
- Add open-output-location affordance.
- Refine empty, processing, cancelled, error, and completion states.
- Complete keyboard and screen-reader review.

Exit condition: the complete V1 workflow is understandable without technical
knowledge.

## Phase 5 — Release readiness

- Expand Rust, frontend, and end-to-end test coverage.
- Test representative and adversarial image fixtures.
- Audit Tauri permissions, CSP, dependencies, and network behavior.
- Validate memory use, concurrency, and large-file behavior.
- Verify packaged macOS and Windows builds.
- Complete legal review of the proprietary freeware terms and document supported
  OS versions.
- Generate and package complete target-specific third-party license inventories
  and texts from the locked release dependencies.
- Prepare signing, notarization, installer, and release documentation.

Exit condition: release candidates meet privacy, safety, quality, and packaging
requirements on both platforms.

### Windows QA checklist

Windows is architecturally supported but is not release-tested. Before claiming
Windows alpha support, verify on a supported Windows installation:

- native multi-file picker and directory picker;
- drag-and-drop paths, including multiple drives;
- thumbnail generation and asset-protocol display;
- JPEG, PNG, WebP, and keep-original output writing;
- overwrite backup, replacement, failure restoration, and cleanup;
- create-copy on NTFS and a removable filesystem without hard-link support;
- per-file failure isolation when the destination disappears or becomes read-only;
- Open Output Folder in Explorer;
- taskbar, Explorer executable, and installer icons at small and large sizes;
- long paths and filenames near Windows limits; and
- Unicode source and destination paths.

### Alpha readiness checklist

- [ ] Complete the macOS manual end-to-end matrix.
- [ ] Confirm there is no known source overwrite or data-loss defect.
- [x] Keep automated frontend and native tests passing.
- [x] Keep branding and runtime/build version metadata aligned.
- [x] Validate safe output behavior in automated tests.
- [x] Close known blocking workflow and accessibility issues.
- [ ] Validate Windows or label the first alpha as macOS-only/not-yet-tested.

## Post-V1 candidates

AVIF, HEIC/HEIF, TIFF, JPEG XL, crop, watermark, metadata inspector, presets,
recursive folder processing, watch folders, upscaling, and comparison tools will
be evaluated after V1. Their order is intentionally undecided.
