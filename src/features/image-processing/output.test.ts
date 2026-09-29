import { describe, expect, it } from "vitest";
import { createDefaultBatchSettings } from "./settings";
import { createCompressSettings, createConvertSettings } from "../workflows/workflow";
import {
  METADATA_BEHAVIOR_MESSAGE,
  createDefaultOutputSettings,
  createSaveModeUpdate,
  createWriteImageRequest,
  isFutureProcessingReady,
  parseWriteImageResult,
  resolveCopyDestination,
  SAVE_MODE_OPTIONS,
} from "./output";

describe("output settings", () => {
  it("defaults to creating copies", () => {
    expect(createDefaultOutputSettings()).toEqual({
      outputDirectory: null,
      saveMode: "createCopies",
    });
  });

  it("exposes destination-oriented V1 save modes", () => {
    expect(SAVE_MODE_OPTIONS).toEqual([
      { value: "createCopies", label: "Create Copies" },
      { value: "replaceOriginals", label: "Replace Originals" },
    ]);
  });

  it("captures a conflict selection before the browser clears the event target", () => {
    let event: { currentTarget: { value: string } | null } = {
      currentTarget: { value: "replaceOriginals" },
    };
    const update = createSaveModeUpdate(event.currentTarget!.value);
    event.currentTarget = null;

    expect(update(createDefaultOutputSettings()).saveMode).toBe("replaceOriginals");
  });

  it("uses an automatic folder for same-directory sources and requires a choice for mixed sources", () => {
    const settings = createDefaultBatchSettings();
    const output = createDefaultOutputSettings();
    expect(isFutureProcessingReady(1, settings, output)).toBe(false);
    const automatic = resolveCopyDestination(["/images/photo.jpg", "/images/logo.png"], null);
    expect(automatic).toEqual({
      path: "/images/PixChisel Copies",
      preflightPath: "/images",
      automatic: true,
    });
    expect(isFutureProcessingReady(2, settings, output, automatic)).toBe(true);
    expect(resolveCopyDestination(["/one/photo.jpg", "/two/logo.png"], null)).toBeNull();
    expect(
      isFutureProcessingReady(1, settings, {
        ...output,
        outputDirectory: "/images/output",
      }, resolveCopyDestination([], "/images/output")),
    ).toBe(true);
    expect(
      isFutureProcessingReady(0, settings, {
        ...output,
        outputDirectory: "/images/output",
      }, resolveCopyDestination([], "/images/output")),
    ).toBe(false);
    expect(isFutureProcessingReady(1, settings, {
      ...output,
      saveMode: "replaceOriginals",
    })).toBe(true);
  });

  it("serializes directory and replace-original destination shapes", () => {
    const settings = createDefaultBatchSettings();
    const selected = resolveCopyDestination([], "/images/output");
    expect(
      createWriteImageRequest("/images/source.png", settings, {
        outputDirectory: "/images/output",
        saveMode: "createCopies",
      }, "compress", selected),
    ).toEqual({
      sourcePath: "/images/source.png",
      settings,
      destination: { mode: "directory", path: "/images/output" },
      operation: "compress",
    });
    expect(createWriteImageRequest("/images/source.png", settings, {
      outputDirectory: null,
      saveMode: "replaceOriginals",
    }, "resize")).toEqual({
      sourcePath: "/images/source.png",
      settings,
      destination: { mode: "replaceOriginal" },
      operation: "resize",
    });
    expect(createWriteImageRequest("/images/source.png", settings, {
      outputDirectory: null,
      saveMode: "createCopies",
    }, "compress")).toBeNull();
  });

  it("serializes an automatic directory without treating it as a selected folder", () => {
    const settings = createDefaultBatchSettings();
    const output = createDefaultOutputSettings();
    const automatic = resolveCopyDestination(["C:\\images\\photo.jpg"], null);
    expect(automatic).toEqual({
      path: "C:\\images\\PixChisel Copies",
      preflightPath: "C:\\images",
      automatic: true,
    });
    expect(createWriteImageRequest("C:\\images\\photo.jpg", settings, output, "compress", automatic))
      .toMatchObject({ destination: { mode: "automaticDirectory", path: automatic?.path } });
  });

  it("preserves destination shapes through every workflow", () => {
    for (const settings of [createCompressSettings("maximum"), createConvertSettings("webp")]) {
      expect(createWriteImageRequest("/images/photo.jpg", settings, {
        outputDirectory: null,
        saveMode: "replaceOriginals",
      }, "convert")).toMatchObject({ destination: { mode: "replaceOriginal" }, settings, operation: "convert" });
    }
  });

  it("states the current metadata behavior truthfully", () => {
    expect(createDefaultBatchSettings().removeMetadata).toBe(true);
    expect(METADATA_BEHAVIOR_MESSAGE).toContain("Metadata is removed");
  });

  it("parses skipped results without written-only metadata", () => {
    expect(parseWriteImageResult({
      status: "skipped", sourcePath: "/input/photo.jpg", outputPath: "/output/photo.jpg", outputFormat: "jpeg",
    })).toEqual({
      status: "skipped", sourcePath: "/input/photo.jpg", outputPath: "/output/photo.jpg", outputFormat: "jpeg",
    });
  });

  it("parses already-optimized results without output metadata", () => {
    expect(parseWriteImageResult({
      status: "notSmaller", sourcePath: "/input/photo.png", outputFormat: "png",
      originalSizeBytes: 100, candidateSizeBytes: 120,
    })).toEqual({
      status: "notSmaller", sourcePath: "/input/photo.png", outputFormat: "png",
      originalSizeBytes: 100, candidateSizeBytes: 120,
    });
  });

  it("rejects malformed written results before they reach React rendering", () => {
    expect(() => parseWriteImageResult({ status: "written", outputPath: "/output/photo.jpg" }))
      .toThrow("invalid processing result");
  });
});
