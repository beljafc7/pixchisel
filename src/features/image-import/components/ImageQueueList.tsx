import { formatFileSize } from "../format";
import type { ImageQueueItem } from "../types";
import type { FileProcessingState } from "../../image-processing/batch";

interface ImageQueueListProps {
  items: ImageQueueItem[];
  onRemove: (id: string) => void;
  processingStates: Record<string, FileProcessingState>;
  disabled: boolean;
}

export function ImageQueueList({ items, onRemove, processingStates, disabled }: ImageQueueListProps) {
  return (
    <ul className="queue-list" aria-label="Imported images">
      {items.map((item) => (
        <li className={`queue-item queue-item--${item.status}`} key={item.id}>
          <div className={`queue-item__thumbnail queue-item__thumbnail--${item.status}`}>
            {item.status === "ready" && item.thumbnail.status === "ready" && (
              <img src={item.thumbnail.url} alt="" aria-hidden="true" />
            )}
            {item.status === "ready" && item.thumbnail.status === "pending" && (
              <span className="thumbnail-pending" aria-label="Generating preview" />
            )}
            {item.status === "error" ||
            (item.status === "ready" && item.thumbnail.status === "error") ? (
              <span className="thumbnail-placeholder" aria-hidden="true">
                —
              </span>
            ) : null}
          </div>
          <div className="queue-item__file">
            <span className="queue-item__name" title={item.filename}>
              {item.filename}
            </span>
            {item.status === "error" && <span className="queue-item__error">{item.error.message}</span>}
            {item.status === "ready" && (
              <ProcessingDetail state={processingStates[item.path]} />
            )}
          </div>
          {item.status === "ready" ? (
            <>
              <span className="queue-item__format">{item.format.toUpperCase()}</span>
              <span className="queue-item__dimensions">
                {item.width} × {item.height}
              </span>
              <span className="queue-item__size">{formatFileSize(item.fileSizeBytes)}</span>
            </>
          ) : (
            <span className="queue-item__status">Error</span>
          )}
          <button className="icon-button" type="button" onClick={() => onRemove(item.id)} disabled={disabled} aria-label={`Remove ${item.filename}`}>
            ×
          </button>
        </li>
      ))}
    </ul>
  );
}

export function ProcessingDetail({ state }: { state: FileProcessingState | undefined }) {
  if (!state || state.status === "ready") return null;
  switch (state.status) {
    case "processing":
      return (
        <span className="processing-detail processing-detail--processing">
          <span>Chiseling · {stageLabel(state.stage)} {state.percent}%</span>
          <progress value={state.percent} max={100} aria-label={`${stageLabel(state.stage)} ${state.percent}%`} />
        </span>
      );
    case "cancelling":
      return <span className="processing-detail processing-detail--cancelled">Cancelling…</span>;
    case "written": {
      const filename = state.result.outputPath.split(/[\\/]/).pop() ?? state.result.outputPath;
      return (
        <span className="processing-detail processing-detail--written" title={state.result.outputPath}>
          Completed · {filename} · {formatFileSize(state.result.originalSizeBytes)} → {formatFileSize(state.result.outputSizeBytes)}{sizeChange(state.result.originalSizeBytes, state.result.outputSizeBytes)}
        </span>
      );
    }
    case "skipped":
      return <span className="processing-detail processing-detail--skipped">Skipped · destination exists</span>;
    case "notSmaller":
      return <span className="processing-detail processing-detail--skipped">Already optimized · no smaller version produced</span>;
    case "failed":
      return <span className="processing-detail processing-detail--failed">Failed · {state.error.message}</span>;
    case "cancelled":
      return <span className="processing-detail processing-detail--cancelled">Cancelled</span>;
  }
}

function stageLabel(stage: Extract<FileProcessingState, { status: "processing" }>["stage"]): string {
  return stage[0].toUpperCase() + stage.slice(1);
}

function sizeChange(original: number, output: number): string {
  if (original === 0 || original === output) return "";
  const percentage = (Math.abs(original - output) / original) * 100;
  return output < original ? ` · ${percentage.toFixed(1)}% smaller` : ` · ${percentage.toFixed(1)}% larger`;
}
