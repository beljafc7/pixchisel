import { Channel, invoke } from "@tauri-apps/api/core";
import type { ProcessingProgress } from "../../features/image-processing/batch";
import type {
  WriteImageError,
  WriteImageRequest,
  WriteImageResult,
} from "../../features/image-processing/output";

export async function writeTransformedImage(
  request: WriteImageRequest,
  onProgress?: (progress: ProcessingProgress) => void,
): Promise<WriteImageResult> {
  try {
    const channel = new Channel<ProcessingProgress>();
    channel.onmessage = (progress) => onProgress?.(progress);
    return await invoke<WriteImageResult>("write_transformed_image", { request, onProgress: channel });
  } catch (error) {
    throw normalizeWriteImageError(error);
  }
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
