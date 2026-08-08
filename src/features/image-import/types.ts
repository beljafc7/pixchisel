import type { ImageInspection, InspectImageError } from "../../types/image";

interface QueueItemBase {
  id: string;
  path: string;
  filename: string;
  extension: string;
}

export interface ReadyQueueItem extends QueueItemBase, ImageInspection {
  status: "ready";
}

export interface ErrorQueueItem extends QueueItemBase {
  status: "error";
  error: InspectImageError;
}

export type ImageQueueItem = ReadyQueueItem | ErrorQueueItem;
