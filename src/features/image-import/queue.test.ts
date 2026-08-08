import { describe, expect, it } from "vitest";
import type { ImageInspectionResult } from "../../types/image";
import { addInspectionResults, removeQueueItem, validQueueSize } from "./queue";

const readyResult = (path: string, size: number): ImageInspectionResult => ({
  status: "ready",
  image: {
    path,
    filename: path.slice(path.lastIndexOf("/") + 1),
    extension: "png",
    format: "png",
    width: 2,
    height: 3,
    fileSizeBytes: size,
  },
});

describe("image import queue", () => {
  it("does not add the same path twice", () => {
    const queue = addInspectionResults([], [readyResult("/images/a.png", 10)]);
    const unchanged = addInspectionResults(queue, [
      readyResult("/images/a.png", 10),
      readyResult("/other/a.png", 20),
    ]);

    expect(unchanged.map((item) => item.path)).toEqual([
      "/images/a.png",
      "/other/a.png",
    ]);
  });

  it("removes one item without affecting the rest", () => {
    const queue = addInspectionResults([], [
      readyResult("/images/a.png", 10),
      readyResult("/images/b.png", 20),
    ]);

    expect(removeQueueItem(queue, "/images/a.png").map((item) => item.path)).toEqual([
      "/images/b.png",
    ]);
  });

  it("counts only valid file sizes", () => {
    const errorResult: ImageInspectionResult = {
      status: "error",
      path: "/images/broken.png",
      filename: "broken.png",
      extension: "png",
      error: { code: "invalidImage", message: "Invalid image." },
    };
    const queue = addInspectionResults([], [readyResult("/images/a.png", 42), errorResult]);

    expect(validQueueSize(queue)).toBe(42);
  });
});
