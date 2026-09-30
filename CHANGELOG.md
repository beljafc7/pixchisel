# Changelog

All notable changes to PixChisel are documented in this file.

## [0.5.0] - 2026-09-30

### Added

- Local JPEG, PNG, and WebP conversion.
- Standard and Maximum image compression modes.
- Resize by width, height, percentage, or bounding box.
- Batch image and folder import with native thumbnails.
- Create Copies and Replace Originals output workflows.
- Shared output selection for batches from multiple source folders.
- Progress, cancellation, per-file results, and output-folder access.
- Automatic cleanup of temporary thumbnail sessions.
- GPL-3.0-only source distribution and trademark guidance.

### Changed

- Resize now preserves JPEG and PNG physical resolution metadata.
- Width, height, and percentage resize modes allow intentional upscaling.
- The initial application window is 1400 × 1100 pixels.
- Output destination controls and explanations were simplified.

### Security

- Added a restrictive desktop Content Security Policy.
- Restricted the asset protocol to the application thumbnail cache.

[0.5.0]: https://github.com/beljafc7/pixchisel/releases/tag/v0.5.0
