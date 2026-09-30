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

export const AUTOMATIC_OUTPUT_FOLDER_NAME = "PixChisel Copies";

export interface CopyDestination {
  path: string;
  preflightPath: string;
  automatic: boolean;
}

export type WriteDestination =
  | { mode: "directory"; path: string }
  | { mode: "automaticDirectory"; path: string }
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
  "Metadata is removed from transformed files in this version. Resize preserves physical resolution.";

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

export function resetCopyDestination(current: OutputSettings): OutputSettings {
  return { ...current, outputDirectory: null };
}

export function isFutureProcessingReady(
  readyImageCount: number,
  settings: BatchSettings,
  output: OutputSettings,
  copyDestination: CopyDestination | null = null,
): boolean {
  return (
    readyImageCount > 0 &&
    isBatchSettingsValid(settings) &&
    isSaveMode(output.saveMode) &&
    (output.saveMode === "replaceOriginals" || copyDestination !== null)
  );
}

export function resolveCopyDestination(
  sourcePaths: string[],
  selectedDirectory: string | null,
): CopyDestination | null {
  if (selectedDirectory) {
    return {
      path: selectedDirectory,
      preflightPath: selectedDirectory,
      automatic: false,
    };
  }
  if (sourcePaths.length === 0) return null;

  const parents = sourcePaths.map(parentDirectory);
  if (parents.some((parent) => parent === null)) return null;
  const first = parents[0]!;
  if (!parents.every((parent) => parent?.path === first.path)) return null;

  return {
    path: joinPath(first.path, first.separator, AUTOMATIC_OUTPUT_FOLDER_NAME),
    preflightPath: first.path,
    automatic: true,
  };
}

export function createWriteImageRequest(
  sourcePath: string,
  settings: BatchSettings,
  output: OutputSettings,
  operation: WorkflowMode,
  copyDestination: CopyDestination | null = null,
): WriteImageRequest | null {
  if (!isSaveMode(output.saveMode)) {
    return null;
  }
  let destination: WriteDestination;
  if (output.saveMode === "replaceOriginals") {
    destination = { mode: "replaceOriginal" };
  } else {
    if (!copyDestination) return null;
    destination = {
      mode: copyDestination.automatic ? "automaticDirectory" : "directory",
      path: copyDestination.path,
    };
  }
  return {
    sourcePath,
    settings,
    destination,
    operation,
  };
}

function parentDirectory(path: string): { path: string; separator: "/" | "\\" } | null {
  const forward = path.lastIndexOf("/");
  const backward = path.lastIndexOf("\\");
  const index = Math.max(forward, backward);
  if (index < 0) return null;
  const separator = backward > forward ? "\\" : "/";
  if (index === 0) return { path: separator, separator };
  let parent = path.slice(0, index);
  if (/^[A-Za-z]:$/.test(parent)) parent += separator;
  return { path: parent, separator };
}

function joinPath(parent: string, separator: "/" | "\\", child: string): string {
  return parent.endsWith("/") || parent.endsWith("\\")
    ? `${parent}${child}`
    : `${parent}${separator}${child}`;
}

function isSaveMode(value: string): value is SaveMode {
  return value === "createCopies" || value === "replaceOriginals";
}
