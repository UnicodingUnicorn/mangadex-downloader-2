use std::num::ParseFloatError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RangeError {
    #[error("invalid range string supplied")]
    Invalid,
    #[error("invalid number supplied: {0}")]
    Parse(#[from] ParseFloatError),
}

fn parse_volume_chapter(s:&str) -> Result<Option<(f64, Option<f64>)>, ParseFloatError> {
    let mut parts = s.split(':');
    let volume = match parts.next() {
        Some(part) => part.trim().parse::<f64>()?,
        None => return Ok(None),
    };

    let chapter = parts.next()
        .map(|part| part.trim().parse::<f64>())
        .transpose()?;

    Ok(Some((volume, chapter)))
}

#[derive(Debug, Copy, Clone)]
pub struct Range {
    pub volume_start: f64,
    pub volume_end: f64,
    pub chapter_start: Option<f64>,
    pub chapter_end: Option<f64>,
}
impl Range {
    pub fn new(s:&str) -> Result<Self, RangeError> {
        let mut parts = s.split("-");
        let (volume_start, chapter_start) = parse_volume_chapter(parts.next()
            .ok_or(RangeError::Invalid)?)?
            .ok_or(RangeError::Invalid)?;

        let (mut volume_end, mut chapter_end) = (volume_start, chapter_start);
        if let Some(part) = parts.next() {
            let (ev, ec) = parse_volume_chapter(part)?.ok_or(RangeError::Invalid)?;
            volume_end = ev;
            chapter_end = ec;
        }

        Ok(Self {
            volume_start,
            volume_end,
            chapter_start,
            chapter_end,
        })
    }

    pub fn from_str(s:&str) -> Result<Vec<Self>, RangeError> {
        s.split(",")
            .map(|part| Self::new(part))
            .collect::<Result<Vec<Self>, RangeError>>()
    }

    pub fn in_range(&self, volume:&str, chapter:&str) -> bool {
        let v = match volume.parse::<f64>() {
            Ok(v) => v,
            Err(_) => return false,
        };

        let c = match chapter.parse::<f64>() {
            Ok(c) => c,
            Err(_) => return false,
        };

        if self.volume_start > v || self.volume_end < v { // Inclusive
            return false;
        }

        if let Some(cs) = self.chapter_start {
            if cs > c {
                return false;
            }
        }

        if let Some(ce) = self.chapter_end {
            if ce < c {
                return false;
            }
        }

        true
    }
}
