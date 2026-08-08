import { describe, expect, it } from "vitest";
import type { WriteImageResult } from "./output";
import {
  runBoundedBatch,
  summarizeBatch,
  type CancellationToken,
  type FileProcessingState,
} from "./batch";

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
