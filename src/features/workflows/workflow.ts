import {
  createDefaultBatchSettings,
  type BatchSettings,
  type OutputFormat,
  type ResizeMode,
} from "../image-processing/settings";

export type WorkflowMode = "compress" | "convert" | "resize";
export type CompressionPreset = "standard" | "strong" | "maximum";
export type ConversionFormat = Exclude<OutputFormat, "original">;

export const conversionQuality: Record<ConversionFormat, number> = {
  jpeg: 92,
  png: 92,
  webp: 75,
};
export const RESIZE_QUALITY = 92;

export const compressionQuality: Record<CompressionPreset, number> = {
  standard: 82,
  strong: 65,
  maximum: 45,
};

export interface ResizeWorkflowSettings {
  mode: Exclude<ResizeMode, "none">;
  width: number;
  height: number;
  maxWidth: number;
  maxHeight: number;
  percentage: number;
  allowUpscaling: boolean;
}

export function createCompressSettings(preset: CompressionPreset): BatchSettings {
  return {
    ...createDefaultBatchSettings(),
    outputFormat: "original",
    quality: compressionQuality[preset],
  };
}

export function createConvertSettings(format: ConversionFormat): BatchSettings {
  return {
    ...createDefaultBatchSettings(),
    outputFormat: format,
    quality: conversionQuality[format],
  };
}

export function createResizeSettings(resize: ResizeWorkflowSettings): BatchSettings {
  const defaults = createDefaultBatchSettings();
  return {
    ...defaults,
    outputFormat: "original",
    quality: RESIZE_QUALITY,
    resize: {
      mode: resize.mode,
      width: resize.width,
      height: resize.height,
      maxWidth: resize.maxWidth,
      maxHeight: resize.maxHeight,
      percentage: resize.percentage,
    },
    allowUpscaling: resize.allowUpscaling,
  };
}

export const workflowCopy: Record<WorkflowMode, { title: string; headline: string; accent: string; intro: string; description: string; resultVerb: string }> = {
  compress: { title: "Compress Images", headline: "Smaller images.", accent: "Same great quality.", intro: "Compress images without visible quality loss. Everything stays on your device.", description: "Reduce size", resultVerb: "compressed" },
  convert: { title: "Convert Images", headline: "Convert images.", accent: "Any format, instantly.", intro: "Change image formats in batch. PNG, JPEG, WebP — your choice.", description: "Change format", resultVerb: "converted" },
  resize: { title: "Resize Images", headline: "Resize images.", accent: "Perfect dimensions.", intro: "Resize by width, height, fit, or percentage. Non-destructive and fast.", description: "Dimensions", resultVerb: "resized" },
};

export function shouldConfirmWorkflowChange(queueLength: number): boolean {
  return queueLength > 0;
}
