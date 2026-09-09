import { describe, expect, it } from "vitest";
import {
  RESIZE_QUALITY,
  compressionQuality,
  conversionQuality,
  createCompressSettings,
  createConvertSettings,
  createResizeSettings,
  shouldConfirmWorkflowChange,
} from "./workflow";

describe("workflow settings adapters", () => {
  it("maps all compression presets while preserving source format and dimensions", () => {
    expect(compressionQuality).toEqual({ standard: 82, strong: 65, maximum: 45 });
    for (const preset of ["standard", "strong", "maximum"] as const) {
      expect(createCompressSettings(preset)).toMatchObject({
        outputFormat: "original",
        quality: compressionQuality[preset],
        resize: { mode: "none" },
      });
    }
  });

  it("uses the benchmarked per-format conversion quality with no resize", () => {
    expect(conversionQuality).toEqual({ jpeg: 92, png: 92, webp: 75 });
    for (const format of ["jpeg", "png", "webp"] as const) {
      expect(createConvertSettings(format)).toMatchObject({
        outputFormat: format,
        quality: conversionQuality[format],
        resize: { mode: "none" },
        allowUpscaling: false,
      });
    }
  });

  it("preserves source format and high quality for every resize mode", () => {
    for (const mode of ["width", "height", "fit", "percentage"] as const) {
      expect(createResizeSettings({
        mode, width: 800, height: 600, maxWidth: 1200, maxHeight: 900,
        percentage: 50, allowUpscaling: false,
      })).toMatchObject({ outputFormat: "original", quality: RESIZE_QUALITY, resize: { mode } });
    }
  });

  it("confirms only when changing action would clear queued images", () => {
    expect(shouldConfirmWorkflowChange(0)).toBe(false);
    expect(shouldConfirmWorkflowChange(1)).toBe(true);
  });
});
