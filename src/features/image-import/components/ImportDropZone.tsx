import type { RefObject } from "react";

interface ImportDropZoneProps {
  compact?: boolean;
  isActive: boolean;
  isImporting: boolean;
  disabled?: boolean;
  onSelect: () => void;
  buttonRef?: RefObject<HTMLButtonElement | null>;
}

export function ImportDropZone({
  compact = false,
  isActive,
  isImporting,
  disabled = false,
  onSelect,
  buttonRef,
}: ImportDropZoneProps) {
  return (
    <div className={`drop-zone${isActive ? " drop-zone--active" : ""}${compact ? " drop-zone--compact" : ""}`}>
      {!compact && (
        <>
          <div className="drop-zone__icon" aria-hidden="true">
            +
          </div>
          <h1>Drop images here</h1>
          <p>JPG, PNG, and WebP</p>
        </>
      )}
      <button ref={buttonRef} className={compact ? "secondary-button" : "primary-button"} type="button" onClick={onSelect} disabled={isImporting || disabled}>
        {compact ? "Add Images" : isImporting ? "Adding…" : "Select Images"}
      </button>
      {isActive && <span className="drop-zone__overlay">Drop to add images</span>}
    </div>
  );
}
