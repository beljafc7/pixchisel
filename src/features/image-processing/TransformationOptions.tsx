import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useReducer, useRef, useState } from "react";
import {
  cancelProcessing,
  clearProcessingCancellation,
  createCancellationId,
  openOutputFolder,
  preflightOutputDirectory,
  writeTransformedImage,
} from "../../lib/native/output";
import { formatFileSize } from "../image-import/format";
import {
  batchSettingsReducer,
  createDefaultBatchSettings,
  MAX_DIMENSION,
  MAX_PERCENTAGE,
  type ResizeMode,
  type ResizeValueKey,
} from "./settings";
import { isBatchSettingsValid, validateBatchSettings } from "./validation";
import {
  compactOutputDirectory,
  createSaveModeUpdate,
  SAVE_MODE_OPTIONS,
  createDefaultOutputSettings,
  isFutureProcessingReady,
  type OutputSettings,
} from "./output";
import { createWriteImageRequest } from "./output";
import {
  BATCH_CONCURRENCY,
  isTerminalState,
  retainQueuedBatchPaths,
  processingStateFromProgress,
  runBoundedBatch,
  summarizeBatch,
  type CancellationToken,
  type FileProcessingState,
} from "./batch";
import {
  createCompressSettings,
  createConvertSettings,
  createResizeSettings,
  workflowCopy,
  type CompressionPreset,
  type ConversionFormat,
  type WorkflowMode,
} from "../workflows/workflow";
import settingsIcon from "../../assets/settings.svg";
import privacyShieldIcon from "../../assets/privacy-shield.svg";

interface TransformationOptionsProps {
  workflow: WorkflowMode;
  readyPaths: string[];
  queuePaths: string[];
  processingStates: Record<string, FileProcessingState>;
  onBatchStart: (paths: string[], resetAll: boolean) => void;
  onItemState: (path: string, state: FileProcessingState) => void;
  onRunningChange: (running: boolean) => void;
  onResultsInvalidated: () => void;
  workspaceResetVersion: number;
}

const conversionFormats: Array<{ value: ConversionFormat; label: string }> = [
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
  workflow,
  readyPaths,
  queuePaths,
  processingStates,
  onBatchStart,
  onItemState,
  onRunningChange,
  onResultsInvalidated,
  workspaceResetVersion,
}: TransformationOptionsProps) {
  const [draftSettings, dispatch] = useReducer(batchSettingsReducer, undefined, createDefaultBatchSettings);
  const [compressionPreset, setCompressionPreset] = useState<CompressionPreset>("standard");
  const [conversionFormat, setConversionFormat] = useState<ConversionFormat>("jpeg");
  const [output, setOutput] = useState(createDefaultOutputSettings);
  const [folderError, setFolderError] = useState<string | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [isCancelling, setIsCancelling] = useState(false);
  const [lastBatchPaths, setLastBatchPaths] = useState<string[]>([]);
  const [statusMessage, setStatusMessage] = useState("");
  const cancellation = useRef<CancellationToken | null>(null);
  const primaryAction = useRef<HTMLButtonElement>(null);
  const settings = useMemo(() => {
    if (workflow === "compress") return createCompressSettings(compressionPreset);
    if (workflow === "convert") return createConvertSettings(conversionFormat);
    return createResizeSettings({
      mode: draftSettings.resize.mode === "none" ? "width" : draftSettings.resize.mode,
      width: draftSettings.resize.width,
      height: draftSettings.resize.height,
      maxWidth: draftSettings.resize.maxWidth,
      maxHeight: draftSettings.resize.maxHeight,
      percentage: draftSettings.resize.percentage,
      allowUpscaling: draftSettings.allowUpscaling,
    });
  }, [compressionPreset, conversionFormat, draftSettings, workflow]);
  const errors = validateBatchSettings(settings);
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
  const batchComplete =
    !isRunning &&
    lastBatchPaths.length > 0 &&
    completedCount === lastBatchPaths.length &&
    summary.failed === 0 &&
    summary.cancelled === 0;

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
    if (isRunning || paths.length === 0 || !settingsAreValid || !isFutureProcessingReady(paths.length, settings, output)) return;
    if (output.saveMode === "createCopies") {
      const directory = output.outputDirectory;
      if (!directory) return;
      try {
        await preflightOutputDirectory(directory);
        setFolderError(null);
      } catch (error) {
        const message = normalizeOutputError(error);
        setFolderError(message);
        setStatusMessage(message);
        return;
      }
    }
    const token: CancellationToken = {
      cancelled: false,
      nativeId: createCancellationId(),
    };
    cancellation.current = token;
    setIsCancelling(false);
    setIsRunning(true);
    setLastBatchPaths(paths);
    setStatusMessage(`Processing ${paths.length} ${paths.length === 1 ? "image" : "images"}.`);
    onBatchStart(paths, resetAll);
    onRunningChange(true);
    try {
      await runBoundedBatch(
        paths,
        async (path) => {
          const request = createWriteImageRequest(path, settings, output, workflow);
          if (!request) throw { code: "writeFailed", message: "Output settings are incomplete." };
          return writeTransformedImage(
            request,
            (progress) => {
              if (!token.cancelled) {
                onItemState(progress.path, processingStateFromProgress(progress));
              }
            },
            token.nativeId,
          );
        },
        token,
        onItemState,
        BATCH_CONCURRENCY,
      );
    } finally {
      if (token.nativeId) {
        await clearProcessingCancellation(token.nativeId).catch(() => undefined);
      }
      cancellation.current = null;
      setIsCancelling(false);
      setIsRunning(false);
      onRunningChange(false);
      setStatusMessage(
        token.cancelled
          ? "Cancellation complete. Cancelled outputs were not saved."
          : "Batch complete. Review the results below.",
      );
      requestAnimationFrame(() => primaryAction.current?.focus());
    }
  }

  function cancelBatch() {
    if (cancellation.current) {
      cancellation.current.cancelled = true;
      setIsCancelling(true);
      for (const path of lastBatchPaths) {
        const state = processingStates[path];
        if (state?.status === "processing") {
          onItemState(path, { status: "cancelling" });
        } else if (!state || state.status === "ready") {
          onItemState(path, { status: "cancelled" });
        }
      }
      if (cancellation.current.nativeId) {
        void cancelProcessing(cancellation.current.nativeId).catch(() => {
          setStatusMessage("Cancellation requested. Active native work could not be interrupted.");
        });
      }
      setStatusMessage("Cancelling. Active files will stop before saving when safe.");
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

  function updateNumber(key: ResizeValueKey, value: number) {
    updateSettings({ type: "setResizeValue", key, value });
  }

  return (
    <section className="options-panel" aria-labelledby="options-title">
      <div className="options-panel__heading">
        <img src={settingsIcon} alt="" />
        <h2 id="options-title">Export settings</h2>
      </div>

      <div className="options-grid">
        {workflow === "compress" && <fieldset className="option-group option-group--wide">
          <legend>Compression</legend>
          <div className="segmented-control">
            {(["standard", "strong", "maximum"] as const).map((preset) => (
              <label key={preset}>
                <input
                  type="radio"
                  name="compression-preset"
                  value={preset}
                  checked={compressionPreset === preset}
                  disabled={isRunning}
                  onChange={() => { invalidateResults(); setCompressionPreset(preset); }}
                />
                <span>{preset[0].toUpperCase() + preset.slice(1)}</span>
              </label>
            ))}
          </div>
          <p className="field-note">Standard balances size and quality. Strong and Maximum prioritize smaller files. PNG remains lossless.</p>
        </fieldset>}

        {workflow === "convert" && <fieldset className="option-group option-group--wide">
          <legend>Format</legend>
          <div className="segmented-control segmented-control--three">
            {conversionFormats.map((format) => <label key={format.value}>
              <input type="radio" name="conversion-format" value={format.value} checked={conversionFormat === format.value} disabled={isRunning} onChange={() => { invalidateResults(); setConversionFormat(format.value); }} />
              <span>{format.label}</span>
            </label>)}
          </div>
          {conversionFormat === "jpeg" && <p className="field-note">Transparent areas will use a white background.</p>}
        </fieldset>}

        {workflow === "resize" && <fieldset className="option-group option-group--wide">
          <legend>Resize by</legend>
          <div className="resize-row">
            <select
              aria-label="Resize mode"
              value={draftSettings.resize.mode === "none" ? "width" : draftSettings.resize.mode}
              disabled={isRunning}
              onChange={(event) =>
                updateSettings({ type: "setResizeMode", value: event.currentTarget.value as ResizeMode })
              }
            >
              {resizeModes.map((mode) => (
                <option key={mode.value} value={mode.value}>{mode.label}</option>
              ))}
            </select>
            <ResizeFields settings={{ ...draftSettings, resize: { ...draftSettings.resize, mode: draftSettings.resize.mode === "none" ? "width" : draftSettings.resize.mode } }} errors={errors} updateNumber={updateNumber} disabled={isRunning} />
          </div>
          <div className="option-checks">
            <label className="check-control">
              <input
                type="checkbox"
                checked={draftSettings.allowUpscaling}
                disabled={isRunning}
                onChange={(event) =>
                  updateSettings({ type: "setAllowUpscaling", value: event.currentTarget.checked })
                }
              />
              Allow upscaling
            </label>
            <span className="field-note">Smaller images won't be enlarged.</span>
          </div>
        </fieldset>}

        <fieldset className="option-group option-group--wide output-options">
          <legend>Save</legend>
          <SaveDestinationControls
            output={output}
            disabled={isRunning}
            onChooseDirectory={() => void chooseOutputDirectory()}
            onOpenDirectory={() => void showOutputFolder()}
            onSaveModeChange={(value) => {
              const updateSaveMode = createSaveModeUpdate(value);
              invalidateResults();
              setOutput(updateSaveMode);
              setFolderError(null);
            }}
          />
          {folderError && <p className="field-error" role="alert">{folderError}</p>}
        </fieldset>
      </div>

      {lastBatchPaths.length > 0 && (
        <div className="batch-progress" aria-live="polite">
          <div className="batch-progress__labels">
            <span>{isRunning ? `${isCancelling ? "Cancelling" : "Processing"} ${completedCount} of ${lastBatchPaths.length}` : `${completedCount} of ${lastBatchPaths.length} complete`}</span>
            <span>{progressPercentage}%</span>
          </div>
          <progress value={completedCount} max={lastBatchPaths.length} aria-label="Overall batch progress" />
        </div>
      )}

      {!isRunning && lastBatchPaths.length > 0 && completedCount === lastBatchPaths.length && (
        <BatchResults summary={summary} workflow={workflow} />
      )}

      <div className="privacy-note">
        <img src={privacyShieldIcon} alt="" />
        <span><strong>Processed locally</strong>Your images stay private and never get uploaded.</span>
      </div>

      <p className="sr-only" role="status" aria-live="polite" aria-atomic="true">
        {statusMessage}
      </p>

      <div className={`options-panel__action${batchComplete ? " options-panel__action--complete" : ""}`}>
        <span className="action-hint">
          {isRunning
            ? isCancelling ? "Stopping active files safely" : "Active files will finish safely"
            : !settingsAreValid
            ? "Check the highlighted settings"
            : output.saveMode === "createCopies" && !output.outputDirectory
              ? "Choose an output folder to continue"
              : futureProcessingReady
                ? "Ready for batch processing"
                : "No valid images are ready"}
        </span>
        <div className="options-panel__buttons">
          {isRunning && (
            <button className="secondary-button" type="button" onClick={cancelBatch} disabled={isCancelling}>{isCancelling ? "Cancelling…" : "Cancel"}</button>
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
            className={`primary-button${batchComplete ? " primary-button--complete" : ""}`}
            type="button"
            disabled={!futureProcessingReady || isRunning || batchComplete}
            onClick={() => void startBatch(readyPaths, true)}
          >
            {batchComplete
              ? `✓ Done — ${completedCount} ${completedCount === 1 ? "image" : "images"} processed`
              : isRunning
              ? `Chiseling ${completedCount} of ${lastBatchPaths.length}…`
              : `Chisel ${readyImageCount} ${readyImageCount === 1 ? "Image" : "Images"}`}
          </button>
        </div>
      </div>
    </section>
  );
}

interface SaveDestinationControlsProps {
  output: OutputSettings;
  disabled: boolean;
  onChooseDirectory: () => void;
  onOpenDirectory: () => void;
  onSaveModeChange: (value: string) => void;
}

export function SaveDestinationControls({
  output,
  disabled,
  onChooseDirectory,
  onOpenDirectory,
  onSaveModeChange,
}: SaveDestinationControlsProps) {
  return <div className="save-destination">
    <div className="segmented-control segmented-control--two">
      {SAVE_MODE_OPTIONS.map((option) => <label key={option.value}>
        <input
          type="radio"
          name="save-mode"
          value={option.value}
          checked={output.saveMode === option.value}
          disabled={disabled}
          onChange={(event) => onSaveModeChange(event.currentTarget.value)}
        />
        <span>{option.label}</span>
      </label>)}
    </div>
    {output.saveMode === "createCopies" ? <div className="output-folder">
      <button className="secondary-button" type="button" onClick={onChooseDirectory} disabled={disabled}>
        {output.outputDirectory ? "Change Folder" : "Choose Folder"}
      </button>
      <span title={output.outputDirectory ?? undefined}>
        {output.outputDirectory ? compactOutputDirectory(output.outputDirectory) : "No folder selected"}
      </span>
      {output.outputDirectory && <button className="text-button" type="button" disabled={disabled} onClick={onOpenDirectory}>
        Open Output Folder
      </button>}
    </div> : <p className="save-destination__warning" role="note">
      Original files will be replaced. This can't be undone.
    </p>}
  </div>;
}

export function BatchResults({ summary, workflow }: { summary: ReturnType<typeof summarizeBatch>; workflow: WorkflowMode }) {
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
          <p>{summary.written} {summary.written === 1 ? "image" : "images"} {workflowCopy[workflow].resultVerb}</p>
        </div>
        {summary.written > 0 && <strong>{sizeMessage}</strong>}
      </div>
      <div className="batch-results__counts">
        <span><strong>{summary.written}</strong> written</span>
        <span><strong>{summary.skipped}</strong> skipped</span>
        <span><strong>{summary.notSmaller}</strong> already optimized</span>
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
