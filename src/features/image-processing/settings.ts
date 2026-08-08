export const MAX_DIMENSION = 32_768;
export const MAX_PERCENTAGE = 1_000;

export type OutputFormat = "original" | "jpeg" | "png" | "webp";
export type ResizeMode = "none" | "width" | "height" | "fit" | "percentage";

export interface BatchSettings {
  outputFormat: OutputFormat;
  quality: number;
  resize: {
    mode: ResizeMode;
    width: number;
    height: number;
    maxWidth: number;
    maxHeight: number;
    percentage: number;
  };
  allowUpscaling: boolean;
  removeMetadata: boolean;
}

export type ResizeValueKey = Exclude<keyof BatchSettings["resize"], "mode">;

export type BatchSettingsAction =
  | { type: "setOutputFormat"; value: OutputFormat }
  | { type: "setQuality"; value: number }
  | { type: "setResizeMode"; value: ResizeMode }
  | { type: "setResizeValue"; key: ResizeValueKey; value: number }
  | { type: "setAllowUpscaling"; value: boolean }
  | { type: "setRemoveMetadata"; value: boolean };

export function createDefaultBatchSettings(): BatchSettings {
  return {
    outputFormat: "original",
    quality: 82,
    resize: {
      mode: "none",
      width: 1920,
      height: 1080,
      maxWidth: 1920,
      maxHeight: 1080,
      percentage: 50,
    },
    allowUpscaling: false,
    removeMetadata: true,
  };
}

export function batchSettingsReducer(
  settings: BatchSettings,
  action: BatchSettingsAction,
): BatchSettings {
  switch (action.type) {
    case "setOutputFormat":
      return { ...settings, outputFormat: action.value };
    case "setQuality":
      return { ...settings, quality: action.value };
    case "setResizeMode":
      return { ...settings, resize: { ...settings.resize, mode: action.value } };
    case "setResizeValue":
      return { ...settings, resize: { ...settings.resize, [action.key]: action.value } };
    case "setAllowUpscaling":
      return { ...settings, allowUpscaling: action.value };
    case "setRemoveMetadata":
      return { ...settings, removeMetadata: action.value };
  }
}

export function isQualityApplicable(format: OutputFormat): boolean {
  return format === "jpeg" || format === "webp";
}

export function serializeBatchSettings(settings: BatchSettings): BatchSettings {
  return JSON.parse(JSON.stringify(settings)) as BatchSettings;
}
