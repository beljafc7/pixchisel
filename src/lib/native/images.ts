import { invoke } from "@tauri-apps/api/core";
import type { ImageInspection, InspectImageError } from "../../types/image";

const fallbackError: InspectImageError = {
  code: "internal",
  message: "The image could not be inspected.",
};

export async function inspectImage(path: string): Promise<ImageInspection> {
  try {
    return await invoke<ImageInspection>("inspect_image", { path });
  } catch (error) {
    throw normalizeInspectImageError(error);
  }
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
