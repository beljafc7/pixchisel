import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { inspectImage, normalizeInspectImageError } from "../../lib/native/images";
import type { ImageInspection, InspectImageError } from "../../types/image";

const imageFilters = [
  {
    name: "Images",
    extensions: ["jpg", "jpeg", "png", "webp"],
  },
];

export function ImageInspector() {
  const [image, setImage] = useState<ImageInspection | null>(null);
  const [error, setError] = useState<InspectImageError | null>(null);
  const [isInspecting, setIsInspecting] = useState(false);

  async function selectImage() {
    setError(null);
    setIsInspecting(true);
    try {
      const path = await open({
        multiple: false,
        directory: false,
        filters: imageFilters,
      });

      if (!path) {
        return;
      }

      setImage(await inspectImage(path));
    } catch (inspectionError) {
      setImage(null);
      setError(normalizeInspectImageError(inspectionError));
    } finally {
      setIsInspecting(false);
    }
  }

  return (
    <section className="inspector" aria-labelledby="inspector-title">
      <p className="welcome__eyebrow">Phase 1.1</p>
      <h1 id="inspector-title">Inspect an image locally.</h1>
      <p className="welcome__description">
        Choose one JPEG, PNG, or WebP file to verify native image inspection.
      </p>

      <button className="primary-button" type="button" onClick={selectImage} disabled={isInspecting}>
        {isInspecting ? "Inspecting…" : "Select Image"}
      </button>

      <div className="inspection-result" aria-live="polite">
        {error && <p className="inspection-error">{error.message}</p>}
        {image && (
          <dl>
            <div>
              <dt>Filename</dt>
              <dd>{image.filename}</dd>
            </div>
            <div>
              <dt>Format</dt>
              <dd>{image.format.toUpperCase()}</dd>
            </div>
            <div>
              <dt>Dimensions</dt>
              <dd>
                {image.width} × {image.height} px
              </dd>
            </div>
            <div>
              <dt>File size</dt>
              <dd>{formatFileSize(image.fileSizeBytes)}</dd>
            </div>
          </dl>
        )}
        {!error && !image && <p>Select an image to view its file details.</p>}
      </div>
    </section>
  );
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }

  const units = ["KB", "MB", "GB"];
  let value = bytes / 1024;
  let unitIndex = 0;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  return `${value.toFixed(value >= 10 ? 1 : 2)} ${units[unitIndex]}`;
}
