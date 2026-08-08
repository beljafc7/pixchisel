import { describe, expect, it } from "vitest";
import {
  batchSettingsReducer,
  createDefaultBatchSettings,
  isQualityApplicable,
  serializeBatchSettings,
} from "./settings";
import { isBatchSettingsValid, validateBatchSettings } from "./validation";

describe("batch settings", () => {
  it("uses safe, predictable defaults", () => {
    const settings = createDefaultBatchSettings();
    expect(settings).toMatchObject({
      outputFormat: "original",
      quality: 82,
      resize: { mode: "none", width: 1920, height: 1080, maxWidth: 1920, maxHeight: 1080, percentage: 50 },
      allowUpscaling: false,
      removeMetadata: false,
    });
    expect(isBatchSettingsValid(settings)).toBe(true);
  });

  it("switches output format without mutating other settings", () => {
    const original = createDefaultBatchSettings();
    const jpeg = batchSettingsReducer(original, { type: "setOutputFormat", value: "jpeg" });
    expect(jpeg.outputFormat).toBe("jpeg");
    expect(jpeg.resize).toEqual(original.resize);
    expect(original.outputFormat).toBe("original");
  });

  it("applies quality only to JPEG and WebP", () => {
    expect(isQualityApplicable("jpeg")).toBe(true);
    expect(isQualityApplicable("webp")).toBe(true);
    expect(isQualityApplicable("png")).toBe(false);
    expect(isQualityApplicable("original")).toBe(false);
  });

  it("switches resize modes while preserving entered values", () => {
    let settings = createDefaultBatchSettings();
    settings = batchSettingsReducer(settings, { type: "setResizeValue", key: "width", value: 800 });
    settings = batchSettingsReducer(settings, { type: "setResizeMode", value: "width" });
    settings = batchSettingsReducer(settings, { type: "setResizeMode", value: "fit" });
    settings = batchSettingsReducer(settings, { type: "setResizeMode", value: "width" });
    expect(settings.resize).toMatchObject({ mode: "width", width: 800 });
  });

  it("rejects empty, zero, fractional, and unsafe active numeric values", () => {
    const base = { ...createDefaultBatchSettings(), outputFormat: "jpeg" as const, quality: Number.NaN };
    expect(validateBatchSettings(base).quality).toBeDefined();

    const width = { ...base, quality: 82, resize: { ...base.resize, mode: "width" as const, width: 0 } };
    expect(validateBatchSettings(width).width).toBeDefined();
    expect(validateBatchSettings({ ...width, resize: { ...width.resize, width: 1.5 } }).width).toBeDefined();
    expect(validateBatchSettings({ ...width, resize: { ...width.resize, width: 32_769 } }).width).toBeDefined();
  });

  it("keeps the serialized request shape stable", () => {
    expect(serializeBatchSettings(createDefaultBatchSettings())).toEqual({
      outputFormat: "original",
      quality: 82,
      resize: { mode: "none", width: 1920, height: 1080, maxWidth: 1920, maxHeight: 1080, percentage: 50 },
      allowUpscaling: false,
      removeMetadata: false,
    });
  });
});
