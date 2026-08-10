import type { WriteImageError, WriteImageResult } from "./output";

export const BATCH_CONCURRENCY = 2;

export type ProcessingStage = "preparing" | "decoding" | "optimizing" | "encoding" | "saving" | "completed";

export interface ProcessingProgress {
  path: string;
  stage: ProcessingStage;
  percent: 5 | 20 | 45 | 70 | 90 | 100;
}

export type FileProcessingState =
  | { status: "ready" }
  | { status: "processing"; stage: ProcessingStage; percent: number }
  | { status: "cancelling" }
  | { status: "written"; result: Extract<WriteImageResult, { status: "written" }> }
  | { status: "skipped"; result: Extract<WriteImageResult, { status: "skipped" }> }
  | { status: "notSmaller"; result: Extract<WriteImageResult, { status: "notSmaller" }> }
  | { status: "failed"; error: WriteImageError }
  | { status: "cancelled" };

export type TerminalFileState = Exclude<FileProcessingState, { status: "ready" | "processing" | "cancelling" }>;

export interface CancellationToken {
  cancelled: boolean;
  nativeId?: string;
}

export interface BatchSummary {
  total: number;
  attempted: number;
  written: number;
  skipped: number;
  notSmaller: number;
  failed: number;
  cancelled: number;
  originalBytes: number;
  outputBytes: number;
  sizeDifferenceBytes: number;
  sizeDifference: "saved" | "larger" | "unchanged";
  percentageDifference: number;
}

export async function runBoundedBatch(
  paths: string[],
  process: (path: string) => Promise<WriteImageResult>,
  token: CancellationToken,
  onState: (path: string, state: FileProcessingState) => void,
  concurrency = BATCH_CONCURRENCY,
): Promise<TerminalFileState[]> {
  const results: Array<TerminalFileState | undefined> = new Array(paths.length);
  let nextIndex = 0;

  async function worker() {
    while (!token.cancelled) {
      const index = nextIndex;
      if (index >= paths.length) return;
      nextIndex += 1;
      const path = paths[index];
      onState(path, { status: "processing", stage: "preparing", percent: 5 });
      try {
        const result = await process(path);
        const state: TerminalFileState = result.status === "written"
          ? { status: "written", result }
          : result.status === "notSmaller"
            ? { status: "notSmaller", result }
            : { status: "skipped", result };
        results[index] = state;
        onState(path, state);
      } catch (error) {
        const normalized = normalizeWriteError(error);
        if (normalized.code === "cancelled") {
          const state: TerminalFileState = { status: "cancelled" };
          results[index] = state;
          onState(path, state);
          continue;
        }
        const state: TerminalFileState = {
          status: "failed",
          error: normalized,
        };
        results[index] = state;
        onState(path, state);
      }
    }
  }

  const workerCount = Math.max(1, Math.min(concurrency, paths.length));
  await Promise.all(Array.from({ length: workerCount }, () => worker()));

  for (let index = 0; index < paths.length; index += 1) {
    if (!results[index]) {
      const state: TerminalFileState = { status: "cancelled" };
      results[index] = state;
      onState(paths[index], state);
    }
  }

  return results as TerminalFileState[];
}

export function summarizeBatch(states: FileProcessingState[]): BatchSummary {
  let written = 0;
  let skipped = 0;
  let notSmaller = 0;
  let failed = 0;
  let cancelled = 0;
  let originalBytes = 0;
  let outputBytes = 0;

  for (const state of states) {
    switch (state.status) {
      case "written":
        written += 1;
        originalBytes += state.result.originalSizeBytes;
        outputBytes += state.result.outputSizeBytes;
        break;
      case "skipped":
        skipped += 1;
        break;
      case "notSmaller":
        notSmaller += 1;
        break;
      case "failed":
        failed += 1;
        break;
      case "cancelled":
        cancelled += 1;
        break;
      case "ready":
      case "processing":
      case "cancelling":
        break;
    }
  }

  const sizeDifferenceBytes = Math.abs(originalBytes - outputBytes);
  const sizeDifference =
    outputBytes < originalBytes ? "saved" : outputBytes > originalBytes ? "larger" : "unchanged";
  return {
    total: states.length,
    attempted: written + skipped + notSmaller + failed,
    written,
    skipped,
    notSmaller,
    failed,
    cancelled,
    originalBytes,
    outputBytes,
    sizeDifferenceBytes,
    sizeDifference,
    percentageDifference:
      originalBytes === 0 ? 0 : (sizeDifferenceBytes / originalBytes) * 100,
  };
}

export function isTerminalState(state: FileProcessingState | undefined): boolean {
  return Boolean(
    state &&
      (state.status === "written" ||
        state.status === "skipped" ||
        state.status === "notSmaller" ||
        state.status === "failed" ||
        state.status === "cancelled"),
  );
}

export function retainQueuedBatchPaths(batchPaths: string[], queuePaths: string[]): string[] {
  const queued = new Set(queuePaths);
  return batchPaths.filter((path) => queued.has(path));
}

export function processingStateFromProgress(progress: ProcessingProgress): FileProcessingState {
  return { status: "processing", stage: progress.stage, percent: progress.percent };
}

function normalizeWriteError(error: unknown): WriteImageError {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error as WriteImageError;
  }
  return { code: "writeFailed", message: "The image could not be processed." };
}
