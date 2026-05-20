use std::{borrow::Cow, sync::Arc};

use crate::{Error, LuminanceSource};
use anyhow::Result;

/// A simple luma8 source for bytes.
#[derive(Debug)]
pub struct Luma8LuminanceSource {
    /// image dimension in form (x,y)
    dimensions: (u32, u32),
    /// raw data for luma 8
    data: Arc<Vec<u8>>,
}
impl LuminanceSource for Luma8LuminanceSource {
    fn get_row(&'_ self, y: usize) -> Option<Cow<'_, [u8]>> {
        let chunk_size = self.dimensions.0 as usize;
        let row_skip = y;
        let column_skip = 0;
        let column_take = self.dimensions.0 as usize;

        let data_start = (chunk_size * row_skip) + column_skip;
        let data_end = (chunk_size * row_skip) + column_skip + column_take;

        let row = &self.data[data_start..data_end];

        Some(Cow::Borrowed(row))
    }

    fn get_matrix(&self) -> Cow<'_, [u8]> {
        Cow::Borrowed(self.data.as_slice())
    }

    fn get_width(&self) -> usize {
        self.dimensions.0 as usize
    }

    fn get_height(&self) -> usize {
        self.dimensions.1 as usize
    }
}

impl Luma8LuminanceSource {
    pub fn new(source: impl Into<Arc<Vec<u8>>>, width: u32, height: u32) -> Result<Self> {
        let data = source.into();
        let expected = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| Error::InvalidArgument {
                message: format!(
                    "Luma8LuminanceSource::new: image dimensions overflow usize ({width} x {height})"
                ),
            })?;
        if data.len() != expected {
            return Err(Error::InvalidArgument {
                message: format!(
                    "Luma8LuminanceSource::new: luma length {} != width*height ({expected})",
                    data.len()
                ),
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
/// pyramid-layer step for `try_harder`-style multi-resolution decoding
/// (mirrors zxing-cpp's `tryDownscale`).
pub fn downscale_luma_buffer(
    src: &[u8],
    width: u32,
    height: u32,
    factor: u32,
) -> Result<(Vec<u8>, u32, u32)> {
    if factor == 0 {
        return Err(Error::InvalidArgument {
            message: format!("downscale_luma_buffer: factor must be at least 1 (got {factor})"),
        }
        .into());
    }
    let expected = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| Error::InvalidArgument {
            message: format!(
                "downscale_luma_buffer: image dimensions overflow usize ({width} x {height})"
            ),
        })?;
    if src.len() != expected {
        return Err(Error::InvalidArgument {
            message: format!(
                "downscale_luma_buffer: src.len() {} must equal width * height ({expected})",
                src.len()
            ),
        }
        .into());
    }
    let new_w = width / factor;
    let new_h = height / factor;
    if factor == 1 {
        return Ok((src.to_vec(), new_w, new_h));
    }
    let factor_us = factor as usize;
    let w_us = width as usize;
    let new_w_us = new_w as usize;
    let mut out = vec![0u8; new_w_us * new_h as usize];
    let half_area = (factor_us * factor_us) / 2;
    for dy in 0..new_h as usize {
        for dx in 0..new_w_us {
            let mut sum = half_area;
            for ty in 0..factor_us {
                for tx in 0..factor_us {
                    sum += src[(dy * factor_us + ty) * w_us + (dx * factor_us + tx)] as usize;
                }
            }
            out[dy * new_w_us + dx] = (sum / (factor_us * factor_us)) as u8;
        }
    }
    Ok((out, new_w, new_h))
}
