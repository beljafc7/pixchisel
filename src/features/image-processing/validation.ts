import {
  MAX_DIMENSION,
  MAX_PERCENTAGE,
  isQualityApplicable,
  type BatchSettings,
  type ResizeValueKey,
} from "./settings";

export type BatchSettingsErrors = Partial<Record<"quality" | ResizeValueKey, string>>;

export function validateBatchSettings(settings: BatchSettings): BatchSettingsErrors {
  const errors: BatchSettingsErrors = {};

  if (isQualityApplicable(settings.outputFormat)) {
    validateNumber(errors, "quality", settings.quality, 1, 100, "Quality");
  }

  switch (settings.resize.mode) {
    case "width":
      validateNumber(errors, "width", settings.resize.width, 1, MAX_DIMENSION, "Width");
      break;
    case "height":
      validateNumber(errors, "height", settings.resize.height, 1, MAX_DIMENSION, "Height");
      break;
    case "fit":
      validateNumber(errors, "maxWidth", settings.resize.maxWidth, 1, MAX_DIMENSION, "Max width");
      validateNumber(errors, "maxHeight", settings.resize.maxHeight, 1, MAX_DIMENSION, "Max height");
      break;
    case "percentage":
      validateNumber(
        errors,
        "percentage",
        settings.resize.percentage,
        1,
        MAX_PERCENTAGE,
        "Percentage",
      );
      break;
    case "none":
      break;
  }

  return errors;
}

export function isBatchSettingsValid(settings: BatchSettings): boolean {
  return Object.keys(validateBatchSettings(settings)).length === 0;
}

function validateNumber(
  errors: BatchSettingsErrors,
  key: "quality" | ResizeValueKey,
  value: number,
  minimum: number,
  maximum: number,
  label: string,
) {
  if (!Number.isFinite(value) || !Number.isInteger(value)) {
    errors[key] = `${label} must be a whole number.`;
  } else if (value < minimum || value > maximum) {
    errors[key] = `${label} must be between ${minimum.toLocaleString()} and ${maximum.toLocaleString()}.`;
  }
}
