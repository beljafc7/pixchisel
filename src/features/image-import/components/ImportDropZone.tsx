import type { RefObject } from "react";

interface ImportDropZoneProps {
  compact?: boolean;
  isActive: boolean;
  isImporting: boolean;
  disabled?: boolean;
  onSelect: () => void;
  onSelectFolder: () => void;
  buttonRef?: RefObject<HTMLButtonElement | null>;
}

export function ImportDropZone({
  compact = false,
  isActive,
  isImporting,
  disabled = false,
  onSelect,
  onSelectFolder,
  buttonRef,
}: ImportDropZoneProps) {
  return (
    <div className={`drop-zone${isActive ? " drop-zone--active" : ""}${compact ? " drop-zone--compact" : ""}`}>
      {!compact && (
        <>
          <div className="drop-zone__icon" aria-hidden="true">
            +
          </div>
          <h1>Drop images or folders here</h1>
          <p>JPG, PNG, and WebP</p>
        </>
      )}
      <div className="drop-zone__actions">
        <button ref={buttonRef} className={compact ? "secondary-button" : "primary-button"} type="button" onClick={onSelect} disabled={isImporting || disabled}>
          {compact ? "Add Images" : isImporting ? "Adding…" : "Select Images"}
        </button>
        <button className="secondary-button" type="button" onClick={onSelectFolder} disabled={isImporting || disabled}>
          {compact ? "Add Folder" : "Select Folder"}
        </button>
      </div>
      {isActive && <span className="drop-zone__overlay">Drop images or folders to add</span>}
    </div>
  );
}
