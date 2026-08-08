import { open } from "@tauri-apps/plugin-dialog";
import { ImportDropZone } from "./components/ImportDropZone";
import { ImageQueueList } from "./components/ImageQueueList";
import { formatFileSize } from "./format";
import { validQueueSize } from "./queue";
import { useImageImportQueue } from "./useImageImportQueue";
import { useNativeFileDrop } from "./useNativeFileDrop";

const imageFilters = [
  {
    name: "Images",
    extensions: ["jpg", "jpeg", "png", "webp"],
  },
];

export function ImageImportQueue() {
  const {
    queue,
    importPaths,
    removeItem,
    clearQueue,
    reportImportError,
    isImporting,
    importError,
  } = useImageImportQueue();
  const isDragActive = useNativeFileDrop(importPaths);

  async function selectImages() {
    try {
      const paths = await open({
        multiple: true,
        directory: false,
        filters: imageFilters,
      });

      if (paths) {
        await importPaths(paths);
      }
    } catch (error) {
      reportImportError(error);
    }
  }

  if (queue.length === 0) {
    return (
      <section className="import-workspace import-workspace--empty" aria-label="Import images">
        <ImportDropZone
          isActive={isDragActive}
          isImporting={isImporting}
          onSelect={selectImages}
        />
        {importError && <p className="workspace-error">{importError}</p>}
        <p className="local-note">Files are inspected locally and never uploaded.</p>
      </section>
    );
  }

  const totalSize = validQueueSize(queue);

  return (
    <section className="import-workspace" aria-labelledby="queue-title">
      <div className="queue-header">
        <div>
          <p className="welcome__eyebrow">Image queue</p>
          <h1 id="queue-title">Ready to chisel</h1>
        </div>
        <div className="queue-header__actions">
          <ImportDropZone
            compact
            isActive={isDragActive}
            isImporting={isImporting}
            onSelect={selectImages}
          />
          <button
            className="text-button"
            type="button"
            onClick={clearQueue}
            disabled={isImporting}
          >
            Clear All
          </button>
        </div>
      </div>

      {importError && <p className="workspace-error">{importError}</p>}
      <ImageQueueList items={queue} onRemove={removeItem} />

      <footer className="queue-summary" aria-live="polite">
        <span>
          {queue.length} {queue.length === 1 ? "file" : "files"} • {formatFileSize(totalSize)}
        </span>
        {isImporting && <span>Adding images…</span>}
      </footer>
    </section>
  );
}
