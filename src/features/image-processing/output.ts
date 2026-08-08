import type { BatchSettings } from "./settings";
import { isBatchSettingsValid } from "./validation";

export type ConflictPolicy = "overwrite" | "createCopy" | "skip";

export const V1_CONFLICT_OPTIONS = [
  { value: "createCopy", label: "Create Copy" },
  { value: "overwrite", label: "Replace Existing" },
] as const satisfies ReadonlyArray<{ value: ConflictPolicy; label: string }>;

export interface OutputSettings {
  outputDirectory: string | null;
  conflictPolicy: ConflictPolicy;
}

export interface WriteImageRequest {
  sourcePath: string;
  outputDirectory: string;
  settings: BatchSettings;
  conflictPolicy: ConflictPolicy;
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

export type WriteImageResult = WrittenImageResult | SkippedImageResult;

export type WriteImageErrorCode =
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
    conflictPolicy: "createCopy",
  };
}

export function createConflictPolicyUpdate(value: string): (current: OutputSettings) => OutputSettings {
  if (!isConflictPolicy(value)) {
    throw new Error("Unknown conflict policy.");
  }
  return (current) => ({ ...current, conflictPolicy: value });
}

export function isFutureProcessingReady(
  readyImageCount: number,
  settings: BatchSettings,
  output: OutputSettings,
): boolean {
  return (
    readyImageCount > 0 &&
    isBatchSettingsValid(settings) &&
    output.outputDirectory !== null &&
    isConflictPolicy(output.conflictPolicy)
  );
}

export function createWriteImageRequest(
  sourcePath: string,
  settings: BatchSettings,
  output: OutputSettings,
): WriteImageRequest | null {
  if (!output.outputDirectory || !isConflictPolicy(output.conflictPolicy)) {
    return null;
  }
  return {
    sourcePath,
    outputDirectory: output.outputDirectory,
    settings,
    conflictPolicy: output.conflictPolicy,
  };
}

export function compactOutputDirectory(directory: string): string {
  const normalized = directory.replace(/[\\/]+$/, "");
  const finalSegment = normalized.split(/[\\/]/).pop();
  return finalSegment || directory;
}

function isConflictPolicy(value: string): value is ConflictPolicy {
  return value === "overwrite" || value === "createCopy" || value === "skip";
}
