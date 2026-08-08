import { open } from "@tauri-apps/plugin-dialog";
import { useReducer, useState } from "react";
import {
  batchSettingsReducer,
  createDefaultBatchSettings,
  isQualityApplicable,
  MAX_DIMENSION,
  MAX_PERCENTAGE,
  type OutputFormat,
  type ResizeMode,
  type ResizeValueKey,
} from "./settings";
import { isBatchSettingsValid, validateBatchSettings } from "./validation";
import {
  METADATA_BEHAVIOR_MESSAGE,
  compactOutputDirectory,
  createDefaultOutputSettings,
  isFutureProcessingReady,
  type ConflictPolicy,
} from "./output";

interface TransformationOptionsProps {
  readyImageCount: number;
}

const outputFormats: Array<{ value: OutputFormat; label: string }> = [
  { value: "original", label: "Keep original" },
  { value: "jpeg", label: "JPEG" },
  { value: "png", label: "PNG" },
  { value: "webp", label: "WebP" },
];

const resizeModes: Array<{ value: ResizeMode; label: string }> = [
  { value: "none", label: "No resize" },
  { value: "width", label: "Width" },
  { value: "height", label: "Height" },
  { value: "fit", label: "Fit within" },
  { value: "percentage", label: "Percentage" },
];

export function TransformationOptions({ readyImageCount }: TransformationOptionsProps) {
  const [settings, dispatch] = useReducer(batchSettingsReducer, undefined, createDefaultBatchSettings);
  const [output, setOutput] = useState(createDefaultOutputSettings);
  const [folderError, setFolderError] = useState<string | null>(null);
  const errors = validateBatchSettings(settings);
  const qualityApplies = isQualityApplicable(settings.outputFormat);
  const settingsAreValid = isBatchSettingsValid(settings);
  const futureProcessingReady = isFutureProcessingReady(readyImageCount, settings, output);

  async function chooseOutputDirectory() {
    try {
      const directory = await open({ directory: true, multiple: false });
      if (directory) {
        setOutput((current) => ({ ...current, outputDirectory: directory }));
        setFolderError(null);
      }
    } catch {
      setFolderError("The output folder could not be selected.");
    }
  }

  function numberValue(value: number): number | "" {
    return Number.isNaN(value) ? "" : value;
  }

  function updateNumber(key: ResizeValueKey, value: number) {
    dispatch({ type: "setResizeValue", key, value });
  }

  return (
    <section className="options-panel" aria-labelledby="options-title">
      <div className="options-panel__heading">
        <div>
          <p className="welcome__eyebrow">Batch settings</p>
          <h2 id="options-title">Transformation options</h2>
        </div>
        <span className="options-panel__scope">Applies to all ready images</span>
      </div>

      <div className="options-grid">
        <fieldset className="option-group">
          <legend>Output format</legend>
          <div className="segmented-control">
            {outputFormats.map((format) => (
              <label key={format.value}>
                <input
                  type="radio"
                  name="output-format"
                  value={format.value}
                  checked={settings.outputFormat === format.value}
                  onChange={() => dispatch({ type: "setOutputFormat", value: format.value })}
                />
                <span>{format.label}</span>
              </label>
            ))}
          </div>
          {settings.outputFormat === "jpeg" && (
            <p className="field-note">Transparent areas will use a white background.</p>
          )}
        </fieldset>

        <fieldset className="option-group">
          <legend>Quality</legend>
          <div className="quality-control">
            <input
              type="range"
              min="1"
              max="100"
              value={numberValue(settings.quality)}
              disabled={!qualityApplies}
              aria-label="Quality"
              onChange={(event) =>
                dispatch({ type: "setQuality", value: event.currentTarget.valueAsNumber })
              }
            />
            <NumericInput
              label="Quality value"
              value={settings.quality}
              min={1}
              max={100}
              disabled={!qualityApplies}
              error={errors.quality}
              suffix="%"
              onChange={(value) => dispatch({ type: "setQuality", value })}
            />
          </div>
          <p className="field-note">
            {qualityApplies ? "Higher quality creates larger files." : "Not used for this format."}
          </p>
        </fieldset>

        <fieldset className="option-group option-group--wide">
          <legend>Resize</legend>
          <div className="resize-row">
            <select
              aria-label="Resize mode"
              value={settings.resize.mode}
              onChange={(event) =>
                dispatch({ type: "setResizeMode", value: event.currentTarget.value as ResizeMode })
              }
            >
              {resizeModes.map((mode) => (
                <option key={mode.value} value={mode.value}>{mode.label}</option>
              ))}
            </select>
            <ResizeFields settings={settings} errors={errors} updateNumber={updateNumber} />
          </div>
          <div className="option-checks">
            <label className="check-control">
              <input
                type="checkbox"
                checked={settings.allowUpscaling}
                disabled={settings.resize.mode === "none"}
                onChange={(event) =>
                  dispatch({ type: "setAllowUpscaling", value: event.currentTarget.checked })
                }
              />
              Allow upscaling
            </label>
            <span className="field-note">Smaller images won't be enlarged.</span>
          </div>
        </fieldset>

        <fieldset className="option-group option-group--wide option-group--metadata">
          <legend>Metadata</legend>
          <p className="field-note metadata-note">{METADATA_BEHAVIOR_MESSAGE}</p>
        </fieldset>

        <fieldset className="option-group option-group--wide output-options">
          <legend>Output</legend>
          <div className="output-options__row">
            <div className="output-folder">
              <button className="secondary-button" type="button" onClick={chooseOutputDirectory}>
                {output.outputDirectory ? "Change Folder" : "Choose Folder"}
              </button>
              <span title={output.outputDirectory ?? undefined}>
                {output.outputDirectory
                  ? compactOutputDirectory(output.outputDirectory)
                  : "No folder selected"}
              </span>
            </div>
            <label className="conflict-control">
              <span>When a file exists</span>
              <select
                value={output.conflictPolicy}
                onChange={(event) =>
                  setOutput((current) => ({
                    ...current,
                    conflictPolicy: event.currentTarget.value as ConflictPolicy,
                  }))
                }
              >
                <option value="createCopy">Create Copy</option>
                <option value="overwrite">Overwrite</option>
                <option value="skip">Skip</option>
              </select>
            </label>
          </div>
          {folderError && <p className="field-error">{folderError}</p>}
        </fieldset>
      </div>

      <div className="options-panel__action">
        <span>
          {!settingsAreValid
            ? "Check the highlighted settings"
            : !output.outputDirectory
              ? "Choose an output folder to continue"
              : futureProcessingReady
                ? "Ready for batch processing"
                : "No valid images are ready"}
        </span>
        <button className="primary-button" type="button" disabled title="Batch processing arrives in Phase 2.3">
          Chisel {readyImageCount} {readyImageCount === 1 ? "Image" : "Images"}
        </button>
      </div>
    </section>
  );
}

interface ResizeFieldsProps {
  settings: ReturnType<typeof createDefaultBatchSettings>;
  errors: ReturnType<typeof validateBatchSettings>;
  updateNumber: (key: ResizeValueKey, value: number) => void;
}

function ResizeFields({ settings, errors, updateNumber }: ResizeFieldsProps) {
  const common = { min: 1, max: MAX_DIMENSION };
  switch (settings.resize.mode) {
    case "width":
      return <NumericInput label="Width" value={settings.resize.width} {...common} error={errors.width} suffix="px" onChange={(value) => updateNumber("width", value)} />;
    case "height":
      return <NumericInput label="Height" value={settings.resize.height} {...common} error={errors.height} suffix="px" onChange={(value) => updateNumber("height", value)} />;
    case "fit":
      return <><NumericInput label="Max width" value={settings.resize.maxWidth} {...common} error={errors.maxWidth} suffix="px" onChange={(value) => updateNumber("maxWidth", value)} /><span className="dimension-separator">×</span><NumericInput label="Max height" value={settings.resize.maxHeight} {...common} error={errors.maxHeight} suffix="px" onChange={(value) => updateNumber("maxHeight", value)} /></>;
    case "percentage":
      return <NumericInput label="Percentage" value={settings.resize.percentage} min={1} max={MAX_PERCENTAGE} error={errors.percentage} suffix="%" onChange={(value) => updateNumber("percentage", value)} />;
    case "none":
      return <span className="field-note">Original dimensions will be kept.</span>;
  }
}

interface NumericInputProps {
  label: string;
  value: number;
  min: number;
  max: number;
  suffix: string;
  disabled?: boolean;
  error?: string;
  onChange: (value: number) => void;
}

function NumericInput({ label, value, suffix, error, onChange, ...inputProps }: NumericInputProps) {
  const errorId = `${label.toLowerCase().replace(/ /g, "-")}-error`;
  return (
    <label className={`numeric-field${error ? " numeric-field--error" : ""}`}>
      <span className="sr-only">{label}</span>
      <span className="numeric-field__input">
        <input
          type="number"
          value={Number.isNaN(value) ? "" : value}
          aria-invalid={Boolean(error)}
          aria-describedby={error ? errorId : undefined}
          onChange={(event) => onChange(event.currentTarget.valueAsNumber)}
          {...inputProps}
        />
        <span>{suffix}</span>
      </span>
      {error && <span className="field-error" id={errorId}>{error}</span>}
    </label>
  );
}
