use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use super::settings::OutputFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionUnit {
    Unspecified,
    Inches,
    Centimeters,
    Meters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalResolution {
    pub horizontal: u32,
    pub vertical: u32,
    pub unit: ResolutionUnit,
}

pub fn read_physical_resolution(path: &Path, format: OutputFormat) -> Option<PhysicalResolution> {
    match format {
        OutputFormat::Jpeg => read_jpeg_resolution(path),
        OutputFormat::Png => read_png_resolution(path),
        OutputFormat::Webp | OutputFormat::Original => None,
    }
}

fn read_jpeg_resolution(path: &Path) -> Option<PhysicalResolution> {
    let mut reader = BufReader::new(File::open(path).ok()?);
    let mut jfif_resolution = None;
    let mut signature = [0_u8; 2];
    reader.read_exact(&mut signature).ok()?;
    if signature != [0xff, 0xd8] {
        return None;
    }

    loop {
        let marker = read_jpeg_marker(&mut reader)?;
        if marker == 0xda || marker == 0xd9 {
            return jfif_resolution;
        }
        if marker == 0x01 || (0xd0..=0xd8).contains(&marker) {
            continue;
        }

        let mut length = [0_u8; 2];
        reader.read_exact(&mut length).ok()?;
        let payload_length = u16::from_be_bytes(length).checked_sub(2)? as usize;
        if marker != 0xe0 && marker != 0xe1 {
            reader
                .seek(SeekFrom::Current(i64::try_from(payload_length).ok()?))
                .ok()?;
            continue;
        }

        let mut payload = vec![0_u8; payload_length];
        reader.read_exact(&mut payload).ok()?;
        if marker == 0xe0 {
            jfif_resolution = parse_jfif_resolution(&payload).or(jfif_resolution);
        } else if let Some(resolution) = parse_exif_resolution(&payload) {
            return Some(resolution);
        }
    }
}

fn parse_jfif_resolution(payload: &[u8]) -> Option<PhysicalResolution> {
    if payload.len() < 12 || &payload[..5] != b"JFIF\0" {
        return None;
    }
    let unit = match payload[7] {
        0 => ResolutionUnit::Unspecified,
        1 => ResolutionUnit::Inches,
        2 => ResolutionUnit::Centimeters,
        _ => return None,
    };
    let horizontal = u16::from_be_bytes([payload[8], payload[9]]);
    let vertical = u16::from_be_bytes([payload[10], payload[11]]);
    physical_resolution(u32::from(horizontal), u32::from(vertical), unit)
}

fn parse_exif_resolution(payload: &[u8]) -> Option<PhysicalResolution> {
    let tiff = payload.strip_prefix(b"Exif\0\0")?;
    let little_endian = match tiff.get(..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    if read_u16(tiff, 2, little_endian)? != 42 {
        return None;
    }
    let ifd_offset = usize::try_from(read_u32(tiff, 4, little_endian)?).ok()?;
    let entry_count = usize::from(read_u16(tiff, ifd_offset, little_endian)?);
    let mut horizontal = None;
    let mut vertical = None;
    let mut unit = None;

    for index in 0..entry_count {
        let entry = ifd_offset
            .checked_add(2)?
            .checked_add(index.checked_mul(12)?)?;
        let tag = read_u16(tiff, entry, little_endian)?;
        let field_type = read_u16(tiff, entry + 2, little_endian)?;
        let count = read_u32(tiff, entry + 4, little_endian)?;
        match (tag, field_type, count) {
            (0x011a, 5, 1) => horizontal = read_exif_rational(tiff, entry + 8, little_endian),
            (0x011b, 5, 1) => vertical = read_exif_rational(tiff, entry + 8, little_endian),
            (0x0128, 3, 1) => {
                unit = match read_u16(tiff, entry + 8, little_endian)? {
                    2 => Some(ResolutionUnit::Inches),
                    3 => Some(ResolutionUnit::Centimeters),
                    _ => None,
                }
            }
            _ => {}
        }
    }

    physical_resolution(horizontal?, vertical?, unit?)
}

fn read_exif_rational(tiff: &[u8], value_offset: usize, little_endian: bool) -> Option<u32> {
    let offset = usize::try_from(read_u32(tiff, value_offset, little_endian)?).ok()?;
    let numerator = read_u32(tiff, offset, little_endian)?;
    let denominator = read_u32(tiff, offset + 4, little_endian)?;
    if denominator == 0 {
        return None;
    }
    Some(
        (u64::from(numerator) + u64::from(denominator) / 2).checked_div(u64::from(denominator))?
            as u32,
    )
}

fn read_u16(bytes: &[u8], offset: usize, little_endian: bool) -> Option<u16> {
    let value: [u8; 2] = bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?;
    Some(if little_endian {
        u16::from_le_bytes(value)
    } else {
        u16::from_be_bytes(value)
    })
}

fn read_u32(bytes: &[u8], offset: usize, little_endian: bool) -> Option<u32> {
    let value: [u8; 4] = bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?;
    Some(if little_endian {
        u32::from_le_bytes(value)
    } else {
        u32::from_be_bytes(value)
    })
}

fn physical_resolution(
    horizontal: u32,
    vertical: u32,
    unit: ResolutionUnit,
) -> Option<PhysicalResolution> {
    if horizontal == 0 || vertical == 0 {
        return None;
    }
    Some(PhysicalResolution {
        horizontal,
        vertical,
        unit,
    })
}

fn read_jpeg_marker(reader: &mut impl Read) -> Option<u8> {
    let mut byte = [0_u8; 1];
    loop {
        reader.read_exact(&mut byte).ok()?;
        if byte[0] == 0xff {
            break;
        }
    }
    loop {
        reader.read_exact(&mut byte).ok()?;
        if byte[0] != 0xff {
            return Some(byte[0]);
        }
    }
}

fn read_png_resolution(path: &Path) -> Option<PhysicalResolution> {
    let decoder = png::Decoder::new(BufReader::new(File::open(path).ok()?));
    let reader = decoder.read_info().ok()?;
    let dimensions = reader.info().pixel_dims?;
    if dimensions.xppu == 0 || dimensions.yppu == 0 {
        return None;
    }
    Some(PhysicalResolution {
        horizontal: dimensions.xppu,
        vertical: dimensions.yppu,
        unit: match dimensions.unit {
            png::Unit::Unspecified => ResolutionUnit::Unspecified,
            png::Unit::Meter => ResolutionUnit::Meters,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_exif_resolution_without_copying_other_metadata() {
        let mut tiff = vec![b'I', b'I', 42, 0, 8, 0, 0, 0, 3, 0];
        tiff.extend_from_slice(&[0x1a, 0x01, 5, 0, 1, 0, 0, 0, 50, 0, 0, 0]);
        tiff.extend_from_slice(&[0x1b, 0x01, 5, 0, 1, 0, 0, 0, 58, 0, 0, 0]);
        tiff.extend_from_slice(&[0x28, 0x01, 3, 0, 1, 0, 0, 0, 2, 0, 0, 0]);
        tiff.extend_from_slice(&[0, 0, 0, 0]);
        tiff.extend_from_slice(&[44, 1, 0, 0, 1, 0, 0, 0]);
        tiff.extend_from_slice(&[44, 1, 0, 0, 1, 0, 0, 0]);
        let mut payload = b"Exif\0\0".to_vec();
        payload.extend_from_slice(&tiff);

        assert_eq!(
            parse_exif_resolution(&payload),
            Some(PhysicalResolution {
                horizontal: 300,
                vertical: 300,
                unit: ResolutionUnit::Inches,
            })
        );
    }
}
