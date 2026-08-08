import type { WriteImageError, WriteImageResult } from "./output";

export const BATCH_CONCURRENCY = 3;

export type FileProcessingState =
  | { status: "ready" }
  | { status: "processing" }
  | { status: "written"; result: Extract<WriteImageResult, { status: "written" }> }
  | { status: "skipped"; result: Extract<WriteImageResult, { status: "skipped" }> }
  | { status: "failed"; error: WriteImageError }
  | { status: "cancelled" };

export type TerminalFileState = Exclude<FileProcessingState, { status: "ready" | "processing" }>;

export interface CancellationToken {
  cancelled: boolean;
}

export interface BatchSummary {
  total: number;
  attempted: number;
  written: number;
  skipped: number;
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
      onState(path, { status: "processing" });
      try {
        const result = await process(path);
        const state: TerminalFileState =
          result.status === "written"
            ? { status: "written", result }
            : { status: "skipped", result };
        results[index] = state;
        onState(path, state);
      } catch (error) {
        const state: TerminalFileState = {
          status: "failed",
          error: normalizeWriteError(error),
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
      case "failed":
        failed += 1;
        break;
      case "cancelled":
        cancelled += 1;
        break;
      case "ready":
      case "processing":
        break;
    }
  }

  const sizeDifferenceBytes = Math.abs(originalBytes - outputBytes);
  const sizeDifference =
    outputBytes < originalBytes ? "saved" : outputBytes > originalBytes ? "larger" : "unchanged";
  return {
    total: states.length,
    attempted: written + skipped + failed,
    written,
    skipped,
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
        state.status === "failed" ||
        state.status === "cancelled"),
  );
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
