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

export const workflowCopy: Record<WorkflowMode, { title: string; description: string; resultVerb: string }> = {
  compress: { title: "Compress Images", description: "Make images smaller", resultVerb: "compressed" },
  convert: { title: "Convert Images", description: "Change image format", resultVerb: "converted" },
  resize: { title: "Resize Images", description: "Change image dimensions", resultVerb: "resized" },
};

export function shouldConfirmWorkflowChange(queueLength: number): boolean {
  return queueLength > 0;
}
