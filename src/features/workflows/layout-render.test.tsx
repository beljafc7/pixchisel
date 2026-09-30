import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import App from "../../App";
import {
  SaveDestinationControls,
  TransformationOptions,
} from "../image-processing/TransformationOptions";
import { WorkflowSelector } from "./WorkflowSelector";

describe("application layout rendering", () => {
  it("renders the redesigned shell with Compress selected by default", () => {
    const markup = renderToStaticMarkup(<App />);

    expect(markup).toContain("Image optimizer");
    expect(markup).toContain("Fast, private, local");
    expect(markup).toContain("Smaller images.");
    expect(markup).toContain("Same great quality.");
    expect(markup).toContain("Drop images here");
    expect(markup).toContain("Export settings");
    expect(markup).toContain('aria-current="page"');
  });

  it("renders every mode as an accessible sidebar selection", () => {
    for (const mode of ["compress", "convert", "resize"] as const) {
      const markup = renderToStaticMarkup(
        <WorkflowSelector activeMode={mode} disabled={false} onSelect={() => undefined} />,
      );

      expect(markup).toContain(`>${mode[0].toUpperCase()}${mode.slice(1)}<`);
      expect(markup.match(/aria-current="page"/g)).toHaveLength(1);
    }
  });

  it("renders only the four actionable resize modes", () => {
    const markup = renderToStaticMarkup(
      <TransformationOptions
        workflow="resize"
        readyPaths={[]}
        queuePaths={[]}
        processingStates={{}}
        onBatchStart={() => undefined}
        onItemState={() => undefined}
        onRunningChange={() => undefined}
        onResultsInvalidated={() => undefined}
        workspaceResetVersion={0}
      />,
    );

    expect(markup.match(/<option/g)).toHaveLength(4);
    expect(markup).toContain('value="width"');
    expect(markup).toContain('value="height"');
    expect(markup).toContain('value="fit"');
    expect(markup).toContain('value="percentage"');
    expect(markup).not.toContain("No resize");
    expect(markup).not.toContain("Allow upscaling");
    expect(markup).toContain('aria-label="Increase width"');
    expect(markup).toContain('aria-label="Decrease width"');
  });

  it("explains where copies are saved and shows the complete folder path", () => {
    const markup = renderToStaticMarkup(
      <SaveDestinationControls
        output={{ saveMode: "createCopies", outputDirectory: "/Users/example/Pictures/Exports" }}
        copyDestination={{
          path: "/Users/example/Pictures/Exports",
          preflightPath: "/Users/example/Pictures/Exports",
          automatic: false,
        }}
        disabled={false}
        onChooseDirectory={() => undefined}
        onResetCopyDestination={() => undefined}
        onSaveModeChange={() => undefined}
      />,
    );

    expect(markup).toContain("Creates copies in the selected folder.");
    expect(markup).toContain("/Users/example/Pictures/Exports");
  });

  it("warns clearly before replacing original files", () => {
    const markup = renderToStaticMarkup(
      <SaveDestinationControls
        output={{ saveMode: "replaceOriginals", outputDirectory: null }}
        copyDestination={null}
        disabled={false}
        onChooseDirectory={() => undefined}
        onResetCopyDestination={() => undefined}
        onSaveModeChange={() => undefined}
      />,
    );

    expect(markup).toContain("Replaces the original files. This can&#x27;t be undone.");
  });
});
