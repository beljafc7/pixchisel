use super::settings::{BatchSettings, ResizeMode, MAX_DIMENSION};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeError {
    InvalidSourceDimensions,
    DimensionLimitExceeded,
    ArithmeticOverflow,
}

pub fn calculate_target(
    source: Dimensions,
    settings: &BatchSettings,
) -> Result<Dimensions, ResizeError> {
    if source.width == 0 || source.height == 0 {
        return Err(ResizeError::InvalidSourceDimensions);
    }

    let requested = match settings.resize.mode {
        ResizeMode::None => source,
        ResizeMode::Width => Dimensions {
            width: setting_dimension(settings.resize.width)?,
            height: rounded_scale(
                source.height,
                setting_dimension(settings.resize.width)?,
                source.width,
            )?,
        },
        ResizeMode::Height => Dimensions {
            width: rounded_scale(
                source.width,
                setting_dimension(settings.resize.height)?,
                source.height,
            )?,
            height: setting_dimension(settings.resize.height)?,
        },
        ResizeMode::Fit => fit_within(
            source,
            setting_dimension(settings.resize.max_width)?,
            setting_dimension(settings.resize.max_height)?,
        )?,
        ResizeMode::Percentage => Dimensions {
            width: rounded_scale(
                source.width,
                setting_dimension(settings.resize.percentage)?,
                100,
            )?,
            height: rounded_scale(
                source.height,
                setting_dimension(settings.resize.percentage)?,
                100,
            )?,
        },
    };

    let target = if settings.allow_upscaling {
        requested
    } else if requested.width > source.width || requested.height > source.height {
        source
    } else {
        requested
    };

    if target.width > MAX_DIMENSION as u32 || target.height > MAX_DIMENSION as u32 {
        return Err(ResizeError::DimensionLimitExceeded);
    }

    Ok(target)
}

fn setting_dimension(value: i64) -> Result<u32, ResizeError> {
    u32::try_from(value).map_err(|_| ResizeError::ArithmeticOverflow)
}

fn fit_within(
    source: Dimensions,
    max_width: u32,
    max_height: u32,
) -> Result<Dimensions, ResizeError> {
    let width_limited = u64::from(max_width) * u64::from(source.height)
        <= u64::from(max_height) * u64::from(source.width);

    if width_limited {
        Ok(Dimensions {
            width: max_width,
            height: rounded_scale(source.height, max_width, source.width)?,
        })
    } else {
        Ok(Dimensions {
            width: rounded_scale(source.width, max_height, source.height)?,
            height: max_height,
        })
    }
}

// Aspect-derived dimensions use integer half-up rounding; a positive result is
// clamped to one pixel so extreme aspect ratios never collapse to zero.
fn rounded_scale(value: u32, numerator: u32, denominator: u32) -> Result<u32, ResizeError> {
    if denominator == 0 {
        return Err(ResizeError::InvalidSourceDimensions);
    }
    let product = u64::from(value)
        .checked_mul(u64::from(numerator))
        .ok_or(ResizeError::ArithmeticOverflow)?;
    let rounded = product
        .checked_add(u64::from(denominator) / 2)
        .ok_or(ResizeError::ArithmeticOverflow)?
        / u64::from(denominator);
    u32::try_from(rounded.max(1)).map_err(|_| ResizeError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::transform::settings::{OutputFormat, ResizeSettings};

    fn settings(mode: ResizeMode) -> BatchSettings {
        BatchSettings {
            output_format: OutputFormat::Png,
            quality: 82,
            resize: ResizeSettings {
                mode,
                width: 800,
                height: 600,
                max_width: 800,
                max_height: 600,
                percentage: 50,
            },
            allow_upscaling: false,
            remove_metadata: false,
        }
    }

    #[test]
    fn calculates_none_width_and_height_for_landscape_and_portrait() {
        let landscape = Dimensions {
            width: 1600,
            height: 900,
        };
        assert_eq!(
            calculate_target(landscape, &settings(ResizeMode::None)).unwrap(),
            landscape
        );
        assert_eq!(
            calculate_target(landscape, &settings(ResizeMode::Width)).unwrap(),
            Dimensions {
                width: 800,
                height: 450
            }
        );

        let portrait = Dimensions {
            width: 900,
            height: 1600,
        };
        assert_eq!(
            calculate_target(portrait, &settings(ResizeMode::Height)).unwrap(),
            Dimensions {
                width: 338,
                height: 600
            }
        );
    }

    #[test]
    fn fits_without_cropping_and_scales_by_percentage() {
        assert_eq!(
            calculate_target(
                Dimensions {
                    width: 1600,
                    height: 1200
                },
                &settings(ResizeMode::Fit)
            )
            .unwrap(),
            Dimensions {
                width: 800,
                height: 600
            }
        );
        assert_eq!(
            calculate_target(
                Dimensions {
                    width: 1600,
                    height: 900
                },
                &settings(ResizeMode::Fit)
            )
            .unwrap(),
            Dimensions {
                width: 800,
                height: 450
            }
        );
        assert_eq!(
            calculate_target(
                Dimensions {
                    width: 800,
                    height: 600
                },
                &settings(ResizeMode::Percentage)
            )
            .unwrap(),
            Dimensions {
                width: 400,
                height: 300
            }
        );
    }

    #[test]
    fn prevents_or_allows_upscaling() {
        let source = Dimensions {
            width: 400,
            height: 200,
        };
        assert_eq!(
            calculate_target(source, &settings(ResizeMode::Width)).unwrap(),
            source
        );
        let mut upscale = settings(ResizeMode::Width);
        upscale.allow_upscaling = true;
        assert_eq!(
            calculate_target(source, &upscale).unwrap(),
            Dimensions {
                width: 800,
                height: 400
            }
        );
    }

    #[test]
    fn extreme_aspect_ratios_never_round_to_zero() {
        let mut narrow = settings(ResizeMode::Width);
        narrow.resize.width = 1;
        assert_eq!(
            calculate_target(
                Dimensions {
                    width: 32_768,
                    height: 1
                },
                &narrow
            )
            .unwrap(),
            Dimensions {
                width: 1,
                height: 1
            }
        );
    }

    #[test]
    fn refuses_targets_beyond_the_dimension_ceiling() {
        let mut oversized = settings(ResizeMode::Percentage);
        oversized.resize.percentage = 1000;
        oversized.allow_upscaling = true;
        assert_eq!(
            calculate_target(
                Dimensions {
                    width: 4000,
                    height: 10
                },
                &oversized
            ),
            Err(ResizeError::DimensionLimitExceeded)
        );
    }
}
