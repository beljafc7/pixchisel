# Roadmap

The phases below are ordered to validate the riskiest native behavior early while
keeping each increment usable and reviewable. Scope may be adjusted, but V1 and
future features should not be mixed silently.

## Foundation — complete

- Remove starter demo behavior and branding.
- Establish product, architecture, and engineering rules.
- Create the minimal PixChisel application shell.
- Confirm frontend and Rust builds.

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
  transformed outputs explicitly remove transferable metadata.**

### Phase 2.2 — Filesystem boundary — complete

- Select an output directory through a native dialog. **Complete.**
- Define the native single-image write request around the established settings.
  **Complete.**
- Implement overwrite, create-copy, and skip behavior without silent data loss.
  **Complete.**
- Write encoded bytes to a temporary sibling and finalize atomically where the
  platform and destination filesystem permit. **Complete.**
- Surface the metadata-preservation limitation before enabling production output.
  **Complete: V1 transformed outputs explicitly remove metadata.**

### Phase 2.3 — Batch boundary — complete

- Connect the existing Chisel action to bounded native batch orchestration.
  **Complete with three concurrent native writes.**
- Reuse the single-image writer for each ready queue item. **Complete.**
- Add stable job and per-file state without returning encoded bytes to React.
  **Complete.**
- Add progress and cooperative cancellation without weakening temporary-file
  cleanup or conflict guarantees. **Complete at safe file boundaries.**
- Keep individual write failures isolated and report skipped files separately.
  **Complete.**

### Phase 2.4 — Workflow refinement

- Perform focused macOS and Windows end-to-end QA with representative large
  batches and destination filesystems.
- Refine recovery for output-directory removal and disk-full conditions.
- Add an open-output-folder affordance and decide whether completed status should
  reset immediately on settings mutation.
- Review focus movement and screen-reader announcements through completion,
  cancellation, retry, and repeated-batch flows.

Exit condition: one image can be transformed locally with predictable output and
source-file safety.

## Phase 3 — Batch jobs — completed early in Phase 2.3

- Introduce bounded native batch execution. **Complete.**
- Add per-file and overall progress. **Complete.**
- Add cooperative cancellation and temporary-file cleanup. **Complete.**
- Isolate individual file failures. **Complete.**
- Keep the UI responsive for large queues. **Complete by using asynchronous
  native calls with a three-item worker pool.**

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
- Select a license and document supported OS versions.
- Prepare signing, notarization, installer, and release documentation.

Exit condition: release candidates meet privacy, safety, quality, and packaging
requirements on both platforms.

## Post-V1 candidates

AVIF, HEIC/HEIF, TIFF, JPEG XL, crop, watermark, metadata inspector, presets,
recursive folder processing, watch folders, upscaling, and comparison tools will
be evaluated after V1. Their order is intentionally undecided.
