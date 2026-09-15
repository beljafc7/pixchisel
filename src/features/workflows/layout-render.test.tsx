import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import App from "../../App";
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
});
