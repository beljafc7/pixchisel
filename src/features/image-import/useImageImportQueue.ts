import { useCallback, useRef, useState } from "react";
import { inspectImages, normalizeInspectImageError } from "../../lib/native/images";
import { addInspectionResults, removeQueueItem } from "./queue";
import type { ImageQueueItem } from "./types";

export function useImageImportQueue() {
  const [queue, setQueue] = useState<ImageQueueItem[]>([]);
  const [activeImports, setActiveImports] = useState(0);
  const [importError, setImportError] = useState<string | null>(null);
  const knownPaths = useRef(new Set<string>());
  const pendingPaths = useRef(new Set<string>());

  const importPaths = useCallback(async (paths: string[]) => {
    const uniquePaths = [...new Set(paths)].filter(
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
    } catch (error) {
      setImportError(normalizeInspectImageError(error).message);
    } finally {
      uniquePaths.forEach((path) => pendingPaths.current.delete(path));
      setActiveImports((count) => count - 1);
    }
  }, []);

  const removeItem = useCallback((id: string) => {
    knownPaths.current.delete(id);
    setQueue((currentQueue) => removeQueueItem(currentQueue, id));
  }, []);

  const clearQueue = useCallback(() => {
    knownPaths.current.clear();
    setQueue([]);
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
    isImporting: activeImports > 0,
    importError,
  };
}
