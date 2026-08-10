import { Channel, invoke } from "@tauri-apps/api/core";
import type { ProcessingProgress } from "../../features/image-processing/batch";
import type {
  WriteImageError,
  WriteImageRequest,
  WriteImageResult,
} from "../../features/image-processing/output";
import { parseWriteImageResult } from "../../features/image-processing/output";

export async function writeTransformedImage(
  request: WriteImageRequest,
  onProgress?: (progress: ProcessingProgress) => void,
  cancellationId = createCancellationId(),
): Promise<WriteImageResult> {
  try {
    const channel = new Channel<ProcessingProgress>();
    channel.onmessage = (progress) => {
      if (isProcessingProgress(progress)) onProgress?.(progress);
    };
    const result = await invoke<unknown>("write_transformed_image", {
      request,
      onProgress: channel,
      cancellationId,
    });
    return parseWriteImageResult(result);
  } catch (error) {
    throw normalizeWriteImageError(error);
  }
}

export async function cancelProcessing(cancellationId: string): Promise<void> {
  await invoke("cancel_processing", { cancellationId });
}

export async function clearProcessingCancellation(cancellationId: string): Promise<void> {
  await invoke("clear_processing_cancellation", { cancellationId });
}

export function createCancellationId(): string {
  return globalThis.crypto?.randomUUID?.() ??
    `batch-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

const stages = new Map([
  ["preparing", 5], ["decoding", 20], ["optimizing", 45],
  ["encoding", 70], ["saving", 90], ["completed", 100],
]);

function isProcessingProgress(value: unknown): value is ProcessingProgress {
  if (typeof value !== "object" || value === null) return false;
  const progress = value as Partial<ProcessingProgress>;
  return typeof progress.path === "string" && typeof progress.stage === "string" &&
    stages.get(progress.stage) === progress.percent;
}

export async function preflightOutputDirectory(directory: string): Promise<void> {
  try {
    await invoke("preflight_output_directory", { directory });
  } catch (error) {
    throw normalizeWriteImageError(error);
  }
}

export async function openOutputFolder(directory: string): Promise<void> {
  try {
    await invoke("open_output_folder", { directory });
  } catch (error) {
    throw normalizeWriteImageError(error);
  }
}

function normalizeWriteImageError(error: unknown): WriteImageError {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error as WriteImageError;
  }
  return {
    code: "writeFailed",
    message: "The output operation could not be completed.",
  };
}
