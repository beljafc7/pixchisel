import type { ImageInspectionResult } from "../../types/image";
import type { ImageQueueItem } from "./types";

export function addInspectionResults(
  queue: ImageQueueItem[],
  results: ImageInspectionResult[],
): ImageQueueItem[] {
  const existingPaths = new Set(queue.map((item) => item.path));
  const additions: ImageQueueItem[] = [];

  for (const result of results) {
    const path = result.status === "ready" ? result.image.path : result.path;
    if (existingPaths.has(path)) {
      continue;
    }

    existingPaths.add(path);
    additions.push(toQueueItem(result));
  }

  return additions.length === 0 ? queue : [...queue, ...additions];
}

export function removeQueueItem(queue: ImageQueueItem[], id: string): ImageQueueItem[] {
  return queue.filter((item) => item.id !== id);
}

export function validQueueSize(queue: ImageQueueItem[]): number {
  return queue.reduce(
    (total, item) => total + (item.status === "ready" ? item.fileSizeBytes : 0),
    0,
  );
}

function toQueueItem(result: ImageInspectionResult): ImageQueueItem {
  if (result.status === "ready") {
    return {
      ...result.image,
      id: result.image.path,
      status: "ready",
    };
  }

  return {
    id: result.path,
    path: result.path,
    filename: result.filename,
    extension: result.extension,
    status: "error",
    error: result.error,
  };
}
