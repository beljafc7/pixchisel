import { describe, expect, it } from "vitest";
import { createDefaultBatchSettings } from "./settings";
import { createCompressSettings, createConvertSettings } from "../workflows/workflow";
import {
  METADATA_BEHAVIOR_MESSAGE,
  createDefaultOutputSettings,
  createWriteImageRequest,
  isFutureProcessingReady,
} from "./output";

describe("output settings", () => {
  it("defaults conflict handling to create copy", () => {
    expect(createDefaultOutputSettings()).toEqual({
      outputDirectory: null,
      conflictPolicy: "createCopy",
    });
  });

  it("requires an output folder for future processing readiness", () => {
    const settings = createDefaultBatchSettings();
    const output = createDefaultOutputSettings();
    expect(isFutureProcessingReady(1, settings, output)).toBe(false);
    expect(
      isFutureProcessingReady(1, settings, {
        ...output,
        outputDirectory: "/images/output",
      }),
    ).toBe(true);
    expect(
      isFutureProcessingReady(0, settings, {
        ...output,
        outputDirectory: "/images/output",
      }),
    ).toBe(false);
  });

  it("serializes the native write request shape", () => {
    const settings = createDefaultBatchSettings();
    expect(
      createWriteImageRequest("/images/source.png", settings, {
        outputDirectory: "/images/output",
        conflictPolicy: "skip",
      }),
    ).toEqual({
      sourcePath: "/images/source.png",
      outputDirectory: "/images/output",
      settings,
      conflictPolicy: "skip",
    });
  });

  it("preserves every conflict policy through workflow-generated requests", () => {
    for (const conflictPolicy of ["createCopy", "overwrite", "skip"] as const) {
      for (const settings of [createCompressSettings("strong"), createConvertSettings("webp")]) {
        expect(createWriteImageRequest("/images/photo.jpg", settings, {
          outputDirectory: "/output",
          conflictPolicy,
        })).toMatchObject({ conflictPolicy, settings });
      }
    }
  });

  it("states the current metadata behavior truthfully", () => {
    expect(createDefaultBatchSettings().removeMetadata).toBe(true);
    expect(METADATA_BEHAVIOR_MESSAGE).toContain("Metadata is removed");
  });
});
