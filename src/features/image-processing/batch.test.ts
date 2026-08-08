import { describe, expect, it } from "vitest";
import type { WriteImageResult } from "./output";
import {
  runBoundedBatch,
  summarizeBatch,
  retainQueuedBatchPaths,
  processingStateFromProgress,
  type CancellationToken,
  type FileProcessingState,
} from "./batch";

describe("retainQueuedBatchPaths", () => {
  it("removes deleted items so completed summaries cannot retain stale rows", () => {
    expect(retainQueuedBatchPaths(["one", "two", "three"], ["one", "three"])).toEqual([
      "one",
      "three",
    ]);
  });
});

describe("processing progress", () => {
  it("maps a native stage to the identified queue item state", () => {
    const progress = { path: "/images/photo.jpg", stage: "encoding" as const, percent: 70 as const };
    expect(progress.path).toBe("/images/photo.jpg");
    expect(processingStateFromProgress(progress)).toEqual({
      status: "processing",
      stage: "encoding",
      percent: 70,
    });
  });
});

function written(path: string, original: number, output: number): WriteImageResult {
  return {
    status: "written",
    sourcePath: path,
    outputPath: `/output/${path}.jpg`,
    inputFormat: "png",
    outputFormat: "jpeg",
    originalWidth: 10,
    originalHeight: 10,
    outputWidth: 10,
    outputHeight: 10,
    encodedSizeBytes: output,
    metadataDisposition: "removed",
    originalSizeBytes: original,
    outputSizeBytes: output,
  };
}

describe("batch processor", () => {
  it("keeps several hundred results and progress updates accurate", async () => {
    const paths = Array.from({ length: 500 }, (_, index) => `image-${index}`);
    let terminalUpdates = 0;
    const results = await runBoundedBatch(
      paths,
      async (path) => written(path, 20, 10),
      { cancelled: false },
      (_path, state) => {
        if (state.status === "written") terminalUpdates += 1;
      },
      3,
    );

    expect(results).toHaveLength(500);
    expect(terminalUpdates).toBe(500);
    expect(summarizeBatch(results)).toMatchObject({
      total: 500,
      written: 500,
      originalBytes: 10_000,
      outputBytes: 5_000,
      percentageDifference: 50,
    });
  });

  it("bounds concurrency and preserves input result order", async () => {
    let active = 0;
    let maximumActive = 0;
    const token: CancellationToken = { cancelled: false };
    const results = await runBoundedBatch(
      ["a", "b", "c", "d", "e"],
      async (path) => {
        active += 1;
        maximumActive = Math.max(maximumActive, active);
        await Promise.resolve();
        active -= 1;
        return written(path, 10, 5);
      },
      token,
      () => undefined,
      2,
    );
    expect(maximumActive).toBe(2);
    expect(results.map((state) => state.status)).toEqual([
      "written", "written", "written", "written", "written",
    ]);
    expect(results.map((state) => state.status === "written" && state.result.sourcePath)).toEqual([
      "a", "b", "c", "d", "e",
    ]);
  });

  it("isolates a failure and continues later items", async () => {
    const results = await runBoundedBatch(
      ["a", "bad", "c"],
      async (path) => {
        if (path === "bad") throw { code: "transformFailed", message: "Invalid image." };
        return written(path, 10, 5);
      },
      { cancelled: false },
      () => undefined,
      1,
    );
    expect(results.map((state) => state.status)).toEqual(["written", "failed", "written"]);
  });

  it("cancellation lets active files settle and starts no new files", async () => {
    const token: CancellationToken = { cancelled: false };
    const resolvers: Array<() => void> = [];
    const started: string[] = [];
    const promise = runBoundedBatch(
      ["a", "b", "c", "d"],
      (path) => {
        started.push(path);
        return new Promise((resolve) => {
          resolvers.push(() => resolve(written(path, 10, 5)));
        });
      },
      token,
      () => undefined,
      2,
    );
    await Promise.resolve();
    token.cancelled = true;
    resolvers.forEach((resolve) => resolve());
    const results = await promise;
    expect(started).toEqual(["a", "b"]);
    expect(results.map((state) => state.status)).toEqual([
      "written", "written", "cancelled", "cancelled",
    ]);
  });
});

describe("batch summary", () => {
  it("counts terminal states and positive savings", () => {
    const states: FileProcessingState[] = [
      { status: "written", result: written("a", 100, 40) as Extract<WriteImageResult, { status: "written" }> },
      { status: "skipped", result: { status: "skipped", sourcePath: "b", outputPath: "out", outputFormat: "png", originalSizeBytes: 20 } },
      { status: "failed", error: { code: "writeFailed", message: "Failed." } },
      { status: "cancelled" },
    ];
    expect(summarizeBatch(states)).toMatchObject({
      total: 4, attempted: 3, written: 1, skipped: 1, failed: 1, cancelled: 1,
      originalBytes: 100, outputBytes: 40, sizeDifferenceBytes: 60,
      sizeDifference: "saved", percentageDifference: 60,
    });
  });

  it("reports larger output and handles zero original bytes", () => {
    const larger = summarizeBatch([
      { status: "written", result: written("a", 40, 50) as Extract<WriteImageResult, { status: "written" }> },
    ]);
    expect(larger).toMatchObject({ sizeDifference: "larger", sizeDifferenceBytes: 10, percentageDifference: 25 });
    const zero = summarizeBatch([
      { status: "written", result: written("a", 0, 10) as Extract<WriteImageResult, { status: "written" }> },
    ]);
    expect(zero.percentageDifference).toBe(0);
  });
});
