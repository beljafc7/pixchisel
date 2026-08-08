export type SupportedImageFormat = "jpeg" | "png" | "webp";

export interface ImageInspection {
  path: string;
  filename: string;
  extension: string;
  format: SupportedImageFormat;
  width: number;
  height: number;
  fileSizeBytes: number;
}

export type InspectImageErrorCode =
  | "fileNotFound"
  | "permissionDenied"
  | "readFailed"
  | "unsupportedFormat"
  | "invalidImage"
  | "invalidPath"
  | "internal";

export interface InspectImageError {
  code: InspectImageErrorCode;
  message: string;
}

export type ImageInspectionResult =
  | {
      status: "ready";
      image: ImageInspection;
    }
  | {
      status: "error";
      path: string;
      filename: string;
      extension: string;
      error: InspectImageError;
    };
