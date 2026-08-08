use serde::{Deserialize, Serialize};

pub const MAX_DIMENSION: i64 = 32_768;
pub const MAX_PERCENTAGE: i64 = 1_000;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OutputFormat {
    Original,
    Jpeg,
    Png,
    Webp,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResizeMode {
    None,
    Width,
    Height,
    Fit,
    Percentage,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResizeSettings {
    pub mode: ResizeMode,
    pub width: i64,
    pub height: i64,
    pub max_width: i64,
    pub max_height: i64,
    pub percentage: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchSettings {
    pub output_format: OutputFormat,
    pub quality: i64,
    pub resize: ResizeSettings,
    pub allow_upscaling: bool,
    pub remove_metadata: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ValidationErrorCode {
    OutOfRange,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    pub code: ValidationErrorCode,
    pub field: &'static str,
    pub message: String,
}

impl BatchSettings {
    pub fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        if matches!(self.output_format, OutputFormat::Jpeg | OutputFormat::Webp) {
            validate_range(&mut errors, "quality", self.quality, 1, 100);
        }

        match self.resize.mode {
            ResizeMode::None => {}
            ResizeMode::Width => validate_range(
                &mut errors,
                "resize.width",
                self.resize.width,
                1,
                MAX_DIMENSION,
            ),
            ResizeMode::Height => validate_range(
                &mut errors,
                "resize.height",
                self.resize.height,
                1,
                MAX_DIMENSION,
            ),
            ResizeMode::Fit => {
                validate_range(
                    &mut errors,
                    "resize.maxWidth",
                    self.resize.max_width,
                    1,
                    MAX_DIMENSION,
                );
                validate_range(
                    &mut errors,
                    "resize.maxHeight",
                    self.resize.max_height,
                    1,
                    MAX_DIMENSION,
                );
            }
            ResizeMode::Percentage => validate_range(
                &mut errors,
                "resize.percentage",
                self.resize.percentage,
                1,
                MAX_PERCENTAGE,
            ),
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

fn validate_range(
    errors: &mut Vec<ValidationError>,
    field: &'static str,
    value: i64,
    minimum: i64,
    maximum: i64,
) {
    if !(minimum..=maximum).contains(&value) {
        errors.push(ValidationError {
            code: ValidationErrorCode::OutOfRange,
            field,
            message: format!("{field} must be between {minimum} and {maximum}."),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn compatible_json() -> serde_json::Value {
        json!({
            "outputFormat": "original",
            "quality": 82,
            "resize": {
                "mode": "none",
                "width": 1920,
                "height": 1080,
                "maxWidth": 1920,
                "maxHeight": 1080,
                "percentage": 50
            },
            "allowUpscaling": false,
            "removeMetadata": false
        })
    }

    #[test]
    fn typescript_contract_round_trips_without_key_changes() {
        let json = compatible_json();
        let settings: BatchSettings = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(settings).unwrap(), json);
    }

    #[test]
    fn every_enum_value_uses_the_typescript_spelling() {
        assert_eq!(
            serde_json::to_value(OutputFormat::Webp).unwrap(),
            json!("webp")
        );
        assert_eq!(
            serde_json::to_value(OutputFormat::Jpeg).unwrap(),
            json!("jpeg")
        );
        assert_eq!(
            serde_json::to_value(ResizeMode::Percentage).unwrap(),
            json!("percentage")
        );
        assert_eq!(serde_json::to_value(ResizeMode::Fit).unwrap(), json!("fit"));
    }

    #[test]
    fn validates_only_active_fields() {
        let mut settings: BatchSettings = serde_json::from_value(compatible_json()).unwrap();
        settings.quality = -1;
        settings.resize.width = 0;
        assert!(settings.validate().is_ok());

        settings.output_format = OutputFormat::Jpeg;
        settings.resize.mode = ResizeMode::Width;
        let errors = settings.validate().unwrap_err();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].field, "quality");
        assert_eq!(errors[1].field, "resize.width");
    }

    #[test]
    fn rejects_invalid_percentage_and_dimensions() {
        let mut settings: BatchSettings = serde_json::from_value(compatible_json()).unwrap();
        settings.resize.mode = ResizeMode::Percentage;
        settings.resize.percentage = 1001;
        assert_eq!(
            settings.validate().unwrap_err()[0].field,
            "resize.percentage"
        );

        settings.resize.mode = ResizeMode::Fit;
        settings.resize.max_width = 0;
        settings.resize.max_height = 32_769;
        assert_eq!(settings.validate().unwrap_err().len(), 2);
    }

    #[test]
    fn malformed_or_unknown_contract_values_fail_cleanly() {
        let mut json = compatible_json();
        json["outputFormat"] = json!("gif");
        assert!(serde_json::from_value::<BatchSettings>(json).is_err());

        let mut json = compatible_json();
        json["resize"]["mode"] = json!("crop");
        assert!(serde_json::from_value::<BatchSettings>(json).is_err());
    }
}
