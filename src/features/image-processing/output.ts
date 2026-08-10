import type { BatchSettings } from "./settings";
import { isBatchSettingsValid } from "./validation";
import type { WorkflowMode } from "../workflows/workflow";

export type SaveMode = "createCopies" | "replaceOriginals";

export const SAVE_MODE_OPTIONS = [
  { value: "createCopies", label: "Create Copies" },
  { value: "replaceOriginals", label: "Replace Originals" },
] as const satisfies ReadonlyArray<{ value: SaveMode; label: string }>;

export interface OutputSettings {
  outputDirectory: string | null;
  saveMode: SaveMode;
}

export type WriteDestination =
  | { mode: "directory"; path: string }
  | { mode: "replaceOriginal" };

export interface WriteImageRequest {
  sourcePath: string;
  settings: BatchSettings;
  destination: WriteDestination;
  operation: WorkflowMode;
}

export interface WrittenImageResult {
  status: "written";
  sourcePath: string;
  outputPath: string;
  inputFormat: "jpeg" | "png" | "webp";
  outputFormat: "jpeg" | "png" | "webp";
  originalWidth: number;
  originalHeight: number;
  outputWidth: number;
  outputHeight: number;
  encodedSizeBytes: number;
  metadataDisposition: "removed" | "discardedUnsupported";
  originalSizeBytes: number;
  outputSizeBytes: number;
}

export interface SkippedImageResult {
  status: "skipped";
  sourcePath: string;
  outputPath: string;
  outputFormat: "jpeg" | "png" | "webp";
}

export interface NotSmallerImageResult {
  status: "notSmaller";
  sourcePath: string;
  outputFormat: "jpeg" | "png" | "webp";
  originalSizeBytes: number;
  candidateSizeBytes: number;
}

export type WriteImageResult = WrittenImageResult | SkippedImageResult | NotSmallerImageResult;

export type WriteImageErrorCode =
  | "cancelled"
  | "outputDirectoryMissing"
  | "outputDirectoryNotWritable"
  | "destinationConflict"
  | "tempFileCreationFailed"
  | "writeFailed"
  | "finalizeFailed"
  | "cleanupFailed"
  | "unsafeSourceDestination"
  | "transformFailed"
  | "openOutputFolderFailed";

export interface WriteImageError {
  code: WriteImageErrorCode;
  message: string;
}

export function parseWriteImageResult(value: unknown): WriteImageResult {
  if (!isRecord(value) || typeof value.status !== "string") {
    throw invalidResult();
  }
  if (value.status === "skipped") {
    requireStrings(value, ["sourcePath", "outputPath", "outputFormat"]);
    if (!isOutputImageFormat(value.outputFormat)) throw invalidResult();
    return {
      status: "skipped",
      sourcePath: value.sourcePath as string,
      outputPath: value.outputPath as string,
      outputFormat: value.outputFormat,
    };
  }
  if (value.status === "notSmaller") {
    requireStrings(value, ["sourcePath", "outputFormat"]);
    if (!isOutputImageFormat(value.outputFormat)) throw invalidResult();
    if (!["originalSizeBytes", "candidateSizeBytes"].every((key) =>
      typeof value[key] === "number" && Number.isFinite(value[key]))) throw invalidResult();
    return value as unknown as NotSmallerImageResult;
  }
  if (value.status === "written") {
    requireStrings(value, ["sourcePath", "outputPath", "inputFormat", "outputFormat", "metadataDisposition"]);
    const numeric = ["originalWidth", "originalHeight", "outputWidth", "outputHeight", "encodedSizeBytes", "originalSizeBytes", "outputSizeBytes"];
    if (!numeric.every((key) => typeof value[key] === "number" && Number.isFinite(value[key]))) throw invalidResult();
    if (!isOutputImageFormat(value.inputFormat) || !isOutputImageFormat(value.outputFormat)) throw invalidResult();
    if (value.metadataDisposition !== "removed" && value.metadataDisposition !== "discardedUnsupported") throw invalidResult();
    return value as unknown as WrittenImageResult;
  }
  throw invalidResult();
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function requireStrings(value: Record<string, unknown>, keys: string[]): void {
  if (!keys.every((key) => typeof value[key] === "string")) throw invalidResult();
}

function isOutputImageFormat(value: unknown): value is "jpeg" | "png" | "webp" {
  return value === "jpeg" || value === "png" || value === "webp";
}

function invalidResult(): WriteImageError {
  return { code: "writeFailed", message: "PixChisel received an invalid processing result." };
}

export const METADATA_BEHAVIOR_MESSAGE =
  "Metadata is removed from transformed files in this version.";

export function createDefaultOutputSettings(): OutputSettings {
  return {
    outputDirectory: null,
    saveMode: "createCopies",
  };
}

export function createSaveModeUpdate(value: string): (current: OutputSettings) => OutputSettings {
  if (!isSaveMode(value)) {
    throw new Error("Unknown save mode.");
  }
  return (current) => ({ ...current, saveMode: value });
}

export function isFutureProcessingReady(
  readyImageCount: number,
  settings: BatchSettings,
  output: OutputSettings,
): boolean {
  return (
    readyImageCount > 0 &&
    isBatchSettingsValid(settings) &&
    isSaveMode(output.saveMode) &&
    (output.saveMode === "replaceOriginals" || output.outputDirectory !== null)
  );
}

export function createWriteImageRequest(
  sourcePath: string,
  settings: BatchSettings,
  output: OutputSettings,
  operation: WorkflowMode,
): WriteImageRequest | null {
  if (!isSaveMode(output.saveMode)) {
    return null;
  }
  let destination: WriteDestination;
  if (output.saveMode === "replaceOriginals") {
    destination = { mode: "replaceOriginal" };
  } else {
    if (!output.outputDirectory) return null;
    destination = { mode: "directory", path: output.outputDirectory };
  }
  return {
    sourcePath,
    settings,
    destination,
    operation,
  };
}

export function compactOutputDirectory(directory: string): string {
  const normalized = directory.replace(/[\\/]+$/, "");
  const finalSegment = normalized.split(/[\\/]/).pop();
  return finalSegment || directory;
}

function isSaveMode(value: string): value is SaveMode {
  return value === "createCopies" || value === "replaceOriginals";
}
