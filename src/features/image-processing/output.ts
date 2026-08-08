import type { BatchSettings } from "./settings";
import { isBatchSettingsValid } from "./validation";

export type ConflictPolicy = "overwrite" | "createCopy" | "skip";

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
  originalSizeBytes: number;
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
  | "transformFailed";

export interface WriteImageError {
  code: WriteImageErrorCode;
  message: string;
}

export const METADATA_BEHAVIOR_MESSAGE =
  "Metadata is removed from transformed files in this version.";

export function createDefaultOutputSettings(): OutputSettings {
  return {
    outputDirectory: null,
    conflictPolicy: "createCopy",
  };
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
