use std::borrow::Cow;

use crate::{Error, LuminanceSource};
use anyhow::Result;

/// A simple luma8 source for bytes.
#[derive(Debug)]
pub struct Luma8LuminanceSource<'a> {
    /// image dimension in form (x,y)
    dimensions: (usize, usize),
    /// raw data for luma 8
    data: Cow<'a, [u8]>,
}
impl LuminanceSource for Luma8LuminanceSource<'_> {
    fn row(&'_ self, y: usize) -> Option<Cow<'_, [u8]>> {
        let width = self.width();
        let data_start = width.checked_mul(y)?;
        let data_end = data_start.checked_add(width)?;
        self.data.get(data_start..data_end).map(Cow::Borrowed)
    }

    fn matrix(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(self.data.as_ref())
    }

    fn width(&self) -> usize {
        self.dimensions.0
    }

    fn height(&self) -> usize {
        self.dimensions.1
    }
}

impl<'a> Luma8LuminanceSource<'a> {
    pub fn new(source: impl Into<Cow<'a, [u8]>>, width: usize, height: usize) -> Result<Self> {
        let data = source.into();
        let expected = width
            .checked_mul(height)
            .ok_or_else(|| Error::InvalidArgument {
                message: format!(
                    "Luma8LuminanceSource::new: image dimensions overflow usize ({width} x {height})"
                ).into(),
            })?;
        if data.len() != expected {
            return Err(Error::InvalidArgument {
                message: format!(
                    "Luma8LuminanceSource::new: luma length {} != width*height ({expected})",
                    data.len()
                ).into(),
            }
            .into());
        }
        Ok(Self {
            dimensions: (width, height),
            data,
        })
    }
}

/// Box-average downscale a row-major luma buffer by an integer `factor`.
/// New dimensions are `(width / factor, height / factor)`. Trailing edge
/// pixels that don't fit a full `factor × factor` block are dropped (truncating
/// division). `factor` must be ≥ 1; `factor == 1` returns a copy. Useful as a
/// pyramid-layer step for `try_harder`-style multi-resolution QR decoding.
pub fn downscale_luma_buffer<'a>(
    src: &'a [u8],
    width: usize,
    height: usize,
    factor: usize,
) -> Result<(Cow<'a, [u8]>, usize, usize)> {
    if factor == 0 {
        return Err(Error::InvalidArgument {
            message: format!("downscale_luma_buffer: factor must be at least 1 (got {factor})").into(),
        }
        .into());
    }
    let expected = width
        .checked_mul(height)
        .ok_or_else(|| Error::InvalidArgument {
            message: format!(
                "downscale_luma_buffer: image dimensions overflow usize ({width} x {height})"
            ).into(),
        })?;
    if src.len() != expected {
        return Err(Error::InvalidArgument {
            message: format!(
                "downscale_luma_buffer: src.len() {} must equal width * height ({expected})",
                src.len()
            ).into(),
        }
        .into());
    }
    let new_w = width / factor;
    let new_h = height / factor;
    if factor == 1 {
        return Ok((Cow::Borrowed(src), new_w, new_h));
    }
    let mut out = vec![0u8; new_w * new_h];
    let half_area = (factor * factor) / 2;
    for dy in 0..new_h {
        for dx in 0..new_w {
            let mut sum = half_area;
            for ty in 0..factor {
                for tx in 0..factor {
                    sum += usize::from(src[(dy * factor + ty) * width + (dx * factor + tx)]);
                }
            }
            out[dy * new_w + dx] = (sum / (factor * factor)) as u8;
        }
    }
    Ok((Cow::Owned(out), new_w, new_h))
}
