import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ProcessingDetail } from "../image-import/components/ImageQueueList";
import { BatchResults } from "./TransformationOptions";
import { summarizeBatch, type FileProcessingState } from "./batch";
import type { WrittenImageResult } from "./output";

function written(conflict: "createCopy" | "overwrite" = "createCopy"): FileProcessingState {
  const result: WrittenImageResult = {
    status: "written",
    sourcePath: "/input/photo.jpg",
    outputPath: conflict === "overwrite" ? "/output/photo.jpg" : "/output/photo (1).jpg",
    inputFormat: "jpeg",
    outputFormat: "jpeg",
    originalWidth: 100,
    originalHeight: 50,
    outputWidth: 100,
    outputHeight: 50,
    encodedSizeBytes: 400,
    metadataDisposition: "removed",
    originalSizeBytes: 1000,
    outputSizeBytes: 400,
  };
  return { status: "written", result };
}

const skipped: FileProcessingState = {
  status: "skipped",
  result: { status: "skipped", sourcePath: "/input/photo.jpg", outputPath: "/output/photo.jpg", outputFormat: "jpeg" },
};

describe("terminal result rendering", () => {
  it("renders create-copy and overwrite written results without throwing", () => {
    expect(renderToStaticMarkup(<ProcessingDetail state={written("createCopy")} />)).toContain("Completed");
    expect(renderToStaticMarkup(<ProcessingDetail state={written("overwrite")} />)).toContain("photo.jpg");
  });

  it("renders skipped without reading written-only size fields", () => {
    const markup = renderToStaticMarkup(<ProcessingDetail state={skipped} />);
    expect(markup).toContain("Skipped");
    expect(markup).not.toContain("undefined");
  });

  it("renders all-skipped and mixed summaries without throwing", () => {
    const skippedSummary = summarizeBatch([skipped, skipped]);
    const mixedSummary = summarizeBatch([written(), skipped]);
    expect(renderToStaticMarkup(<BatchResults summary={skippedSummary} workflow="compress" />)).toContain("2</strong> skipped");
    expect(renderToStaticMarkup(<BatchResults summary={mixedSummary} workflow="convert" />)).toContain("1</strong> written");
  });
});
