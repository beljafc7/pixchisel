import { workflowCopy, type WorkflowMode } from "./workflow";
import compressIcon from "../../assets/compress.svg";
import compressInactiveIcon from "../../assets/compress-inactive.svg";
import convertIcon from "../../assets/convert.svg";
import convertActiveIcon from "../../assets/convert-active.svg";
import resizeIcon from "../../assets/resize.svg";
import resizeActiveIcon from "../../assets/resize-active.svg";

const workflowIcons = {
  compress: { active: compressIcon, inactive: compressInactiveIcon },
  convert: { active: convertActiveIcon, inactive: convertIcon },
  resize: { active: resizeActiveIcon, inactive: resizeIcon },
};

interface WorkflowSelectorProps {
  activeMode: WorkflowMode;
  disabled: boolean;
  onSelect: (mode: WorkflowMode) => void;
}

export function WorkflowSelector({ activeMode, disabled, onSelect }: WorkflowSelectorProps) {
  return (
    <nav className="workflow-selector" aria-label="Image action">
      <p className="section-label">Mode</p>
      <div className="workflow-selector__choices">
        {(["compress", "convert", "resize"] as const).map((mode) => (
          <button
            key={mode}
            className={`workflow-choice${activeMode === mode ? " workflow-choice--active" : ""}`}
            type="button"
            aria-current={activeMode === mode ? "page" : undefined}
            disabled={disabled}
            onClick={() => onSelect(mode)}
          >
            <img src={activeMode === mode ? workflowIcons[mode].active : workflowIcons[mode].inactive} alt="" />
            <strong>{mode[0].toUpperCase() + mode.slice(1)}</strong>
            <span>{workflowCopy[mode].description}</span>
          </button>
        ))}
      </div>
    </nav>
  );
}
