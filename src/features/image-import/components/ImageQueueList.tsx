import { formatFileSize } from "../format";
import type { ImageQueueItem } from "../types";

interface ImageQueueListProps {
  items: ImageQueueItem[];
  onRemove: (id: string) => void;
}

export function ImageQueueList({ items, onRemove }: ImageQueueListProps) {
  return (
    <ul className="queue-list" aria-label="Imported images">
      {items.map((item) => (
        <li className={`queue-item queue-item--${item.status}`} key={item.id}>
          <div className="queue-item__file">
            <span className="queue-item__name" title={item.filename}>
              {item.filename}
            </span>
            {item.status === "error" && <span className="queue-item__error">{item.error.message}</span>}
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
          <button className="icon-button" type="button" onClick={() => onRemove(item.id)} aria-label={`Remove ${item.filename}`}>
            ×
          </button>
        </li>
      ))}
    </ul>
  );
}
