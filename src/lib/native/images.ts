import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type {
  ImageInspectionResult,
  InspectImageError,
  ThumbnailResult,
} from "../../types/image";

const fallbackError: InspectImageError = {
  code: "internal",
  message: "The image could not be inspected.",
};

export async function inspectImages(paths: string[]): Promise<ImageInspectionResult[]> {
  try {
    return await invoke<ImageInspectionResult[]>("inspect_images", { paths });
  } catch (error) {
    throw normalizeInspectImageError(error);
  }
}

export async function generateThumbnails(paths: string[]): Promise<ThumbnailResult[]> {
  return invoke<ThumbnailResult[]>("generate_thumbnails", { paths });
}

export async function releaseThumbnails(paths: string[]): Promise<void> {
  return invoke("release_thumbnails", { paths });
}

export async function clearThumbnailCache(): Promise<void> {
  return invoke("clear_thumbnail_cache");
}

export function thumbnailUrl(path: string): string {
  return convertFileSrc(path);
}

export function normalizeInspectImageError(error: unknown): InspectImageError {
  if (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error as InspectImageError;
  }

  return fallbackError;
}
