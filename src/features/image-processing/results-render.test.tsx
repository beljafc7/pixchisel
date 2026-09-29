import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ProcessingDetail } from "../image-import/components/ImageQueueList";
import { BatchResults, SaveDestinationControls, TransformationOptions } from "./TransformationOptions";
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

const alreadyOptimized: FileProcessingState = {
  status: "notSmaller",
  result: { status: "notSmaller", sourcePath: "/input/photo.png", outputFormat: "png", originalSizeBytes: 100, candidateSizeBytes: 100 },
};

describe("terminal result rendering", () => {
  it("renders immediate feedback while an active file is cancelling", () => {
    expect(renderToStaticMarkup(<ProcessingDetail state={{ status: "cancelling" }} />)).toContain("Cancelling");
  });

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

  it("renders all and mixed already-optimized results without treating them as failures", () => {
    expect(renderToStaticMarkup(<ProcessingDetail state={alreadyOptimized} />)).toContain("Already optimized");
    const all = renderToStaticMarkup(<BatchResults summary={summarizeBatch([alreadyOptimized])} workflow="compress" />);
    const mixed = renderToStaticMarkup(<BatchResults summary={summarizeBatch([written(), alreadyOptimized])} workflow="compress" />);
    expect(all).toContain("1</strong> already optimized");
    expect(all).toContain("0</strong> failed");
    expect(mixed).toContain("1</strong> written");
    expect(mixed).toContain("1</strong> already optimized");
  });
});

describe("save destination rendering", () => {
  const handlers = {
    disabled: false,
    onChooseDirectory: () => undefined,
    onOpenDirectory: () => undefined,
    onSaveModeChange: () => undefined,
  };

  it("shows and requires an output folder only when creating copies", () => {
    const markup = renderToStaticMarkup(<SaveDestinationControls
      {...handlers}
      output={{ saveMode: "createCopies", outputDirectory: null }}
      copyDestination={{ path: "/images/PixChisel Copies", preflightPath: "/images", automatic: true }}
      canOpenDirectory={false}
    />);
    expect(markup).toContain("Change Folder");
    expect(markup).toContain("PixChisel Copies (automatic)");
    expect(markup).not.toContain("Open Output Folder");
    expect(markup).not.toContain("can&#x27;t be undone");
  });

  it("renders the destructive warning and no folder control when replacing originals", () => {
    const markup = renderToStaticMarkup(<SaveDestinationControls
      {...handlers}
      output={{ saveMode: "replaceOriginals", outputDirectory: "/unused" }}
      copyDestination={{ path: "/unused", preflightPath: "/unused", automatic: false }}
      canOpenDirectory
    />);
    expect(markup).toContain("Original files will be replaced");
    expect(markup).toContain("can&#x27;t be undone");
    expect(markup).not.toContain("Choose Folder");
    expect(markup).not.toContain("Open Output Folder");
  });
});

describe("sidebar content order", () => {
  it("places the local-processing reassurance after the primary action", () => {
    const markup = renderToStaticMarkup(<TransformationOptions
      workflow="compress"
      readyPaths={["/images/photo.jpg"]}
      queuePaths={["/images/photo.jpg"]}
      processingStates={{}}
      onBatchStart={() => undefined}
      onItemState={() => undefined}
      onRunningChange={() => undefined}
      onResultsInvalidated={() => undefined}
      workspaceResetVersion={0}
    />);

    expect(markup.indexOf("Chisel 1 Image")).toBeLessThan(markup.indexOf("Processed locally"));
  });
});
