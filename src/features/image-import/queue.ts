import type { ImageInspectionResult, ThumbnailResult } from "../../types/image";
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

export function validQueueCount(queue: ImageQueueItem[]): number {
  return queue.filter((item) => item.status === "ready").length;
}

export function applyThumbnailResults(
  queue: ImageQueueItem[],
  results: ThumbnailResult[],
  toUrl: (path: string) => string,
): ImageQueueItem[] {
  const resultsByPath = new Map(results.map((result) => [result.path, result]));

  return queue.map((item) => {
    if (item.status !== "ready") {
      return item;
    }

    const result = resultsByPath.get(item.path);
    if (!result) {
      return item;
    }

    return {
      ...item,
      thumbnail:
        result.status === "ready"
          ? { status: "ready", url: toUrl(result.thumbnailPath) }
          : { status: "error", error: result.error },
    };
  });
}

function toQueueItem(result: ImageInspectionResult): ImageQueueItem {
  if (result.status === "ready") {
    return {
      ...result.image,
      id: result.image.path,
      status: "ready",
      thumbnail: { status: "pending" },
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
