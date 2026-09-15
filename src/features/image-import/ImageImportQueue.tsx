import { open } from "@tauri-apps/plugin-dialog";
import { useRef, useState } from "react";
import { ImportDropZone } from "./components/ImportDropZone";
import { ImageQueueList } from "./components/ImageQueueList";
import { formatFileSize } from "./format";
import { TransformationOptions } from "../image-processing/TransformationOptions";
import { validQueueSize } from "./queue";
import { useImageImportQueue } from "./useImageImportQueue";
import { useNativeFileDrop } from "./useNativeFileDrop";
import { isTerminalState, type FileProcessingState } from "../image-processing/batch";
import { shouldConfirmWorkflowChange, workflowCopy, type WorkflowMode } from "../workflows/workflow";
import { WorkflowSelector } from "../workflows/WorkflowSelector";

const imageFilters = [
  {
    name: "Images",
    extensions: ["jpg", "jpeg", "png", "webp"],
  },
];

export function ImageImportQueue({ workflow, onChangeWorkflow }: { workflow: WorkflowMode; onChangeWorkflow: (mode: WorkflowMode) => void }) {
  const [processingStates, setProcessingStates] = useState<Record<string, FileProcessingState>>({});
  const [isBatchRunning, setIsBatchRunning] = useState(false);
  const [workspaceResetVersion, setWorkspaceResetVersion] = useState(0);
  const selectImagesButton = useRef<HTMLButtonElement>(null);
  const {
    queue,
    importPaths,
    removeItem,
    clearQueue,
    reportImportError,
    isImporting,
    isScanning,
    importError,
  } = useImageImportQueue();
  const isDragActive = useNativeFileDrop(importPaths, isBatchRunning);

  async function selectImages() {
    if (isBatchRunning) return;
    try {
      const paths = await open({
        multiple: true,
        directory: false,
        filters: imageFilters,
      });

      if (paths) {
        await importPaths(paths);
        requestAnimationFrame(() => selectImagesButton.current?.focus());
      }
    } catch (error) {
      reportImportError(error);
    }
  }

  async function selectFolder() {
    if (isBatchRunning) return;
    try {
      const directory = await open({ directory: true, multiple: false });
      if (directory) {
        await importPaths([directory]);
        requestAnimationFrame(() => selectImagesButton.current?.focus());
      }
    } catch (error) {
      reportImportError(error);
    }
  }

  function startBatchState(paths: string[], resetAll: boolean) {
    setProcessingStates((current) => {
      const next = resetAll ? {} : { ...current };
      paths.forEach((path) => {
        next[path] = { status: "ready" };
      });
      return next;
    });
  }

  function updateItemState(path: string, state: FileProcessingState) {
    setProcessingStates((current) => {
      if (state.status === "processing" && isTerminalState(current[path])) return current;
      return { ...current, [path]: state };
    });
  }

  function removeQueueItemAndState(id: string) {
    removeItem(id);
    setProcessingStates((current) => {
      const next = { ...current };
      delete next[id];
      return next;
    });
  }

  function clearQueueAndState() {
    clearQueue();
    setProcessingStates({});
    setWorkspaceResetVersion((version) => version + 1);
    requestAnimationFrame(() => selectImagesButton.current?.focus());
  }

  function changeWorkflow(nextWorkflow: WorkflowMode) {
    if (nextWorkflow === workflow) return;
    if (
      shouldConfirmWorkflowChange(queue.length) &&
      !window.confirm("Changing action will clear the current image queue.")
    ) return;
    clearQueueAndState();
    onChangeWorkflow(nextWorkflow);
  }

  const isEmpty = queue.length === 0;
  const totalSize = validQueueSize(queue);

  const copy = workflowCopy[workflow];

  return (
    <section className="import-workspace" aria-label="PixChisel workspace">
      <div className="workspace-main">
        <header className="workspace-hero">
          <h1><span>{copy.headline}</span><strong>{copy.accent}</strong></h1>
          <p>{copy.intro}</p>
        </header>

        {isEmpty ? (
          <div className="empty-workspace">
            <ImportDropZone
              isActive={isDragActive}
              isImporting={isImporting}
              disabled={isBatchRunning}
              onSelect={selectImages}
              onSelectFolder={selectFolder}
              buttonRef={selectImagesButton}
            />
            {importError && <p className="workspace-error">{importError}</p>}
            {isScanning && <p className="local-note" role="status">Scanning folder…</p>}
          </div>
        ) : (
          <div className="queue-workspace">
            <div className="queue-header">
              <div>
                <p className="section-label">Image queue</p>
                <h2 id="queue-title">{copy.title}</h2>
              </div>
              <div className="queue-header__actions">
                <ImportDropZone
                  compact
                  isActive={isDragActive}
                  isImporting={isImporting}
                  disabled={isBatchRunning}
                  onSelect={selectImages}
                  onSelectFolder={selectFolder}
                  buttonRef={selectImagesButton}
                />
                <button className="text-button" type="button" onClick={clearQueueAndState} disabled={isImporting || isBatchRunning}>Clear All</button>
              </div>
            </div>
            {importError && <p className="workspace-error">{importError}</p>}
            <ImageQueueList items={queue} processingStates={processingStates} onRemove={removeQueueItemAndState} disabled={isBatchRunning} />
            <footer className="queue-summary" aria-live="polite">
              <span>{queue.length} {queue.length === 1 ? "file" : "files"} · {formatFileSize(totalSize)}</span>
              {isScanning ? <span>Scanning folder…</span> : isImporting && <span>Adding images…</span>}
            </footer>
          </div>
        )}
      </div>

      <aside className="workspace-sidebar">
        <WorkflowSelector activeMode={workflow} disabled={isBatchRunning} onSelect={changeWorkflow} />
        <TransformationOptions
          workflow={workflow}
          readyPaths={queue.filter((item) => item.status === "ready").map((item) => item.path)}
          queuePaths={queue.map((item) => item.id)}
          processingStates={processingStates}
          onBatchStart={startBatchState}
          onItemState={updateItemState}
          onRunningChange={setIsBatchRunning}
          onResultsInvalidated={() => setProcessingStates({})}
          workspaceResetVersion={workspaceResetVersion}
        />
      </aside>
    </section>
  );
}
