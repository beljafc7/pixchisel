import { useCallback, useRef, useState } from "react";
import {
  clearThumbnailCache,
  discoverImages,
  generateThumbnails,
  inspectImages,
  normalizeInspectImageError,
  releaseThumbnails,
  thumbnailUrl,
} from "../../lib/native/images";
import { addInspectionResults, applyThumbnailResults, removeQueueItem } from "./queue";
import type { ImageQueueItem } from "./types";

export function useImageImportQueue() {
  const [queue, setQueue] = useState<ImageQueueItem[]>([]);
  const [activeImports, setActiveImports] = useState(0);
  const [activeScans, setActiveScans] = useState(0);
  const [importError, setImportError] = useState<string | null>(null);
  const knownPaths = useRef(new Set<string>());
  const pendingPaths = useRef(new Set<string>());

  const loadThumbnails = useCallback(async (paths: string[]) => {
    try {
      const results = await generateThumbnails(paths);
      const releasedPaths = results
        .map((result) => result.path)
        .filter((path) => !knownPaths.current.has(path));

      setQueue((currentQueue) => applyThumbnailResults(currentQueue, results, thumbnailUrl));

      if (releasedPaths.length > 0) {
        await releaseThumbnails(releasedPaths);
      }
    } catch {
      setQueue((currentQueue) =>
        applyThumbnailResults(
          currentQueue,
          paths.map((path) => ({
            status: "error",
            path,
            error: {
              code: "internal",
              message: "A preview could not be generated.",
            },
          })),
          thumbnailUrl,
        ),
      );
    }
  }, []);

  const importPaths = useCallback(async (paths: string[]) => {
    setActiveScans((count) => count + 1);
    setImportError(null);
    let resolvedPaths: string[];
    try {
      resolvedPaths = await discoverImages(paths);
    } catch (error) {
      setImportError(
        typeof error === "object" && error !== null && "message" in error && typeof error.message === "string"
          ? error.message
          : "The selected folder could not be scanned.",
      );
      return;
    } finally {
      setActiveScans((count) => count - 1);
    }

    const uniquePaths = [...new Set(resolvedPaths)].filter(
      (path) => !knownPaths.current.has(path) && !pendingPaths.current.has(path),
    );
    if (uniquePaths.length === 0) {
      return;
    }

    uniquePaths.forEach((path) => pendingPaths.current.add(path));
    setActiveImports((count) => count + 1);
    setImportError(null);

    try {
      const results = await inspectImages(uniquePaths);
      results.forEach((result) => {
        knownPaths.current.add(result.status === "ready" ? result.image.path : result.path);
      });
      setQueue((currentQueue) => addInspectionResults(currentQueue, results));
      const readyPaths = results
        .filter((result) => result.status === "ready")
        .map((result) => result.image.path);
      if (readyPaths.length > 0) {
        void loadThumbnails(readyPaths);
      }
    } catch (error) {
      setImportError(normalizeInspectImageError(error).message);
    } finally {
      uniquePaths.forEach((path) => pendingPaths.current.delete(path));
      setActiveImports((count) => count - 1);
    }
  }, [loadThumbnails]);

  const removeItem = useCallback((id: string) => {
    knownPaths.current.delete(id);
    setQueue((currentQueue) => removeQueueItem(currentQueue, id));
    void releaseThumbnails([id]).catch(() => undefined);
  }, []);

  const clearQueue = useCallback(() => {
    knownPaths.current.clear();
    pendingPaths.current.clear();
    setQueue([]);
    setImportError(null);
    void clearThumbnailCache().catch(() => undefined);
  }, []);

  const reportImportError = useCallback((error: unknown) => {
    setImportError(normalizeInspectImageError(error).message);
  }, []);

  return {
    queue,
    importPaths,
    removeItem,
    clearQueue,
    reportImportError,
    isImporting: activeImports > 0 || activeScans > 0,
    isScanning: activeScans > 0,
    importError,
  };
}
