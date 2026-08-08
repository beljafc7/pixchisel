import type { ImageInspection, InspectImageError, ThumbnailError } from "../../types/image";

interface QueueItemBase {
  id: string;
  path: string;
  filename: string;
  extension: string;
}

export interface ReadyQueueItem extends QueueItemBase, ImageInspection {
  status: "ready";
  thumbnail:
    | { status: "pending" }
    | { status: "ready"; url: string }
    | { status: "error"; error: ThumbnailError };
}

export interface ErrorQueueItem extends QueueItemBase {
  status: "error";
  error: InspectImageError;
}

export type ImageQueueItem = ReadyQueueItem | ErrorQueueItem;
