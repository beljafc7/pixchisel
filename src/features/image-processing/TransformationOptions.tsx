import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useReducer, useRef, useState } from "react";
import {
  openOutputFolder,
  preflightOutputDirectory,
  writeTransformedImage,
} from "../../lib/native/output";
import { formatFileSize } from "../image-import/format";
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
import { createWriteImageRequest } from "./output";
import {
  BATCH_CONCURRENCY,
  isTerminalState,
  retainQueuedBatchPaths,
  runBoundedBatch,
  summarizeBatch,
  type CancellationToken,
  type FileProcessingState,
} from "./batch";

interface TransformationOptionsProps {
  readyPaths: string[];
  queuePaths: string[];
  processingStates: Record<string, FileProcessingState>;
  onBatchStart: (paths: string[], resetAll: boolean) => void;
  onItemState: (path: string, state: FileProcessingState) => void;
  onRunningChange: (running: boolean) => void;
  onResultsInvalidated: () => void;
  workspaceResetVersion: number;
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

export function TransformationOptions({
  readyPaths,
  queuePaths,
  processingStates,
  onBatchStart,
  onItemState,
  onRunningChange,
  onResultsInvalidated,
  workspaceResetVersion,
}: TransformationOptionsProps) {
  const [settings, dispatch] = useReducer(batchSettingsReducer, undefined, createDefaultBatchSettings);
  const [output, setOutput] = useState(createDefaultOutputSettings);
  const [folderError, setFolderError] = useState<string | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [lastBatchPaths, setLastBatchPaths] = useState<string[]>([]);
  const [statusMessage, setStatusMessage] = useState("");
  const cancellation = useRef<CancellationToken | null>(null);
  const primaryAction = useRef<HTMLButtonElement>(null);
  const errors = validateBatchSettings(settings);
  const qualityApplies = isQualityApplicable(settings.outputFormat);
  const settingsAreValid = isBatchSettingsValid(settings);
  const readyImageCount = readyPaths.length;
  const futureProcessingReady = isFutureProcessingReady(readyImageCount, settings, output);
  const completedCount = lastBatchPaths.filter((path) => isTerminalState(processingStates[path])).length;
  const progressPercentage = lastBatchPaths.length === 0
    ? 0
    : Math.round((completedCount / lastBatchPaths.length) * 100);
  const retryPaths = lastBatchPaths.filter((path) => {
    const status = processingStates[path]?.status;
    return status === "failed" || status === "cancelled";
  });
  const summary = useMemo(
    () => summarizeBatch(lastBatchPaths.map((path) => processingStates[path] ?? { status: "ready" })),
    [lastBatchPaths, processingStates],
  );

  useEffect(() => {
    setLastBatchPaths((paths) => retainQueuedBatchPaths(paths, queuePaths));
  }, [queuePaths]);

  useEffect(() => {
    setLastBatchPaths([]);
    setStatusMessage("");
  }, [workspaceResetVersion]);

  function invalidateResults() {
    if (lastBatchPaths.length > 0) {
      setLastBatchPaths([]);
      setStatusMessage("Previous results cleared because batch settings changed.");
      onResultsInvalidated();
    }
  }

  function updateSettings(action: Parameters<typeof batchSettingsReducer>[1]) {
    invalidateResults();
    dispatch(action);
  }

  async function startBatch(paths: string[], resetAll: boolean) {
    if (isRunning || !output.outputDirectory || paths.length === 0 || !settingsAreValid) return;
    try {
      await preflightOutputDirectory(output.outputDirectory);
      setFolderError(null);
    } catch (error) {
      const message = normalizeOutputError(error);
      setFolderError(message);
      setStatusMessage(message);
      return;
    }
    const token: CancellationToken = { cancelled: false };
    cancellation.current = token;
    setIsRunning(true);
    setLastBatchPaths(paths);
    setStatusMessage(`Processing ${paths.length} ${paths.length === 1 ? "image" : "images"}.`);
    onBatchStart(paths, resetAll);
    onRunningChange(true);
    try {
      await runBoundedBatch(
        paths,
        async (path) => {
          const request = createWriteImageRequest(path, settings, output);
          if (!request) throw { code: "writeFailed", message: "Output settings are incomplete." };
          return writeTransformedImage(request);
        },
        token,
        onItemState,
        BATCH_CONCURRENCY,
      );
    } finally {
      cancellation.current = null;
      setIsRunning(false);
      onRunningChange(false);
      setStatusMessage("Batch complete. Review the results below.");
      requestAnimationFrame(() => primaryAction.current?.focus());
    }
  }

  function cancelBatch() {
    if (cancellation.current) {
      cancellation.current.cancelled = true;
      setStatusMessage("Cancellation requested. Active files will finish safely.");
    }
  }

  async function chooseOutputDirectory() {
    if (isRunning) return;
    try {
      const directory = await open({ directory: true, multiple: false });
      if (directory) {
        invalidateResults();
        setOutput((current) => ({ ...current, outputDirectory: directory }));
        setFolderError(null);
      }
    } catch {
      setFolderError("The output folder could not be selected.");
    }
  }

  async function showOutputFolder() {
    if (!output.outputDirectory) return;
    try {
      await openOutputFolder(output.outputDirectory);
      setFolderError(null);
    } catch (error) {
      const message = normalizeOutputError(error);
      setFolderError(message);
      setStatusMessage(message);
    }
  }

  function numberValue(value: number): number | "" {
    return Number.isNaN(value) ? "" : value;
  }

  function updateNumber(key: ResizeValueKey, value: number) {
    updateSettings({ type: "setResizeValue", key, value });
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
                  disabled={isRunning}
                  onChange={() => updateSettings({ type: "setOutputFormat", value: format.value })}
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
              disabled={!qualityApplies || isRunning}
              aria-label="Quality"
              onChange={(event) =>
                updateSettings({ type: "setQuality", value: event.currentTarget.valueAsNumber })
              }
            />
            <NumericInput
              label="Quality value"
              value={settings.quality}
              min={1}
              max={100}
              disabled={!qualityApplies || isRunning}
              error={errors.quality}
              suffix="%"
              onChange={(value) => updateSettings({ type: "setQuality", value })}
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
              disabled={isRunning}
              onChange={(event) =>
                updateSettings({ type: "setResizeMode", value: event.currentTarget.value as ResizeMode })
              }
            >
              {resizeModes.map((mode) => (
                <option key={mode.value} value={mode.value}>{mode.label}</option>
              ))}
            </select>
            <ResizeFields settings={settings} errors={errors} updateNumber={updateNumber} disabled={isRunning} />
          </div>
          <div className="option-checks">
            <label className="check-control">
              <input
                type="checkbox"
                checked={settings.allowUpscaling}
                disabled={settings.resize.mode === "none" || isRunning}
                onChange={(event) =>
                  updateSettings({ type: "setAllowUpscaling", value: event.currentTarget.checked })
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
              <button className="secondary-button" type="button" onClick={chooseOutputDirectory} disabled={isRunning}>
                {output.outputDirectory ? "Change Folder" : "Choose Folder"}
              </button>
              <span title={output.outputDirectory ?? undefined}>
                {output.outputDirectory
                  ? compactOutputDirectory(output.outputDirectory)
                  : "No folder selected"}
              </span>
              {output.outputDirectory && (
                <button className="text-button" type="button" disabled={isRunning} onClick={() => void showOutputFolder()}>
                  Open Output Folder
                </button>
              )}
            </div>
            <label className="conflict-control">
              <span>When a file exists</span>
              <select
                value={output.conflictPolicy}
                disabled={isRunning}
                onChange={(event) => {
                  invalidateResults();
                  setOutput((current) => ({
                    ...current,
                    conflictPolicy: event.currentTarget.value as ConflictPolicy,
                  }));
                }}
              >
                <option value="createCopy">Create Copy</option>
                <option value="overwrite">Overwrite</option>
                <option value="skip">Skip</option>
              </select>
            </label>
          </div>
          {folderError && <p className="field-error" role="alert">{folderError}</p>}
        </fieldset>
      </div>

      {lastBatchPaths.length > 0 && (
        <div className="batch-progress" aria-live="polite">
          <div className="batch-progress__labels">
            <span>{isRunning ? `Processing ${completedCount} of ${lastBatchPaths.length}` : `${completedCount} of ${lastBatchPaths.length} complete`}</span>
            <span>{progressPercentage}%</span>
          </div>
          <progress value={completedCount} max={lastBatchPaths.length} aria-label="Overall batch progress" />
        </div>
      )}

      {!isRunning && lastBatchPaths.length > 0 && completedCount === lastBatchPaths.length && (
        <BatchResults summary={summary} />
      )}

      <p className="sr-only" role="status" aria-live="polite" aria-atomic="true">
        {statusMessage}
      </p>

      <div className="options-panel__action">
        <span>
          {isRunning
            ? "Active files will finish safely"
            : !settingsAreValid
            ? "Check the highlighted settings"
            : !output.outputDirectory
              ? "Choose an output folder to continue"
              : futureProcessingReady
                ? "Ready for batch processing"
                : "No valid images are ready"}
        </span>
        <div className="options-panel__buttons">
          {isRunning && (
            <button className="secondary-button" type="button" onClick={cancelBatch}>Cancel</button>
          )}
          {!isRunning && retryPaths.length > 0 && (
            <button
              className="secondary-button"
              type="button"
              disabled={!isFutureProcessingReady(retryPaths.length, settings, output)}
              onClick={() => void startBatch(retryPaths, false)}
            >
              Retry Failed &amp; Cancelled
            </button>
          )}
          <button
            ref={primaryAction}
            className="primary-button"
            type="button"
            disabled={!futureProcessingReady || isRunning}
            onClick={() => void startBatch(readyPaths, true)}
          >
            {isRunning
              ? `Chiseling ${completedCount} of ${lastBatchPaths.length}…`
              : `Chisel ${readyImageCount} ${readyImageCount === 1 ? "Image" : "Images"}`}
          </button>
        </div>
      </div>
    </section>
  );
}

function BatchResults({ summary }: { summary: ReturnType<typeof summarizeBatch> }) {
  const sizeMessage = summary.sizeDifference === "saved"
    ? `${formatFileSize(summary.sizeDifferenceBytes)} saved (${summary.percentageDifference.toFixed(1)}%)`
    : summary.sizeDifference === "larger"
      ? `${formatFileSize(summary.sizeDifferenceBytes)} larger`
      : "No size change";
  return (
    <section className="batch-results" aria-labelledby="batch-results-title" tabIndex={-1}>
      <div className="batch-results__heading">
        <div>
          <h3 id="batch-results-title">Chisel complete</h3>
          <p>{summary.total} {summary.total === 1 ? "image" : "images"} in this run</p>
        </div>
        {summary.written > 0 && <strong>{sizeMessage}</strong>}
      </div>
      <div className="batch-results__counts">
        <span><strong>{summary.written}</strong> written</span>
        <span><strong>{summary.skipped}</strong> skipped</span>
        <span><strong>{summary.failed}</strong> failed</span>
        <span><strong>{summary.cancelled}</strong> cancelled</span>
      </div>
      {summary.written > 0 && (
        <p>Written files: {formatFileSize(summary.originalBytes)} → {formatFileSize(summary.outputBytes)}</p>
      )}
    </section>
  );
}

function normalizeOutputError(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error.message;
  }
  return "The output folder is unavailable. Choose it again and retry.";
}

interface ResizeFieldsProps {
  settings: ReturnType<typeof createDefaultBatchSettings>;
  errors: ReturnType<typeof validateBatchSettings>;
  updateNumber: (key: ResizeValueKey, value: number) => void;
  disabled: boolean;
}

function ResizeFields({ settings, errors, updateNumber, disabled }: ResizeFieldsProps) {
  const common = { min: 1, max: MAX_DIMENSION };
  switch (settings.resize.mode) {
    case "width":
      return <NumericInput label="Width" value={settings.resize.width} {...common} disabled={disabled} error={errors.width} suffix="px" onChange={(value) => updateNumber("width", value)} />;
    case "height":
      return <NumericInput label="Height" value={settings.resize.height} {...common} disabled={disabled} error={errors.height} suffix="px" onChange={(value) => updateNumber("height", value)} />;
    case "fit":
      return <><NumericInput label="Max width" value={settings.resize.maxWidth} {...common} disabled={disabled} error={errors.maxWidth} suffix="px" onChange={(value) => updateNumber("maxWidth", value)} /><span className="dimension-separator">×</span><NumericInput label="Max height" value={settings.resize.maxHeight} {...common} disabled={disabled} error={errors.maxHeight} suffix="px" onChange={(value) => updateNumber("maxHeight", value)} /></>;
    case "percentage":
      return <NumericInput label="Percentage" value={settings.resize.percentage} min={1} max={MAX_PERCENTAGE} disabled={disabled} error={errors.percentage} suffix="%" onChange={(value) => updateNumber("percentage", value)} />;
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
