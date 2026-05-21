use anyhow::{Context, Result, ensure};

use crate::Error;

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Matrix<T: Default + Copy> {
    width: usize,
    height: usize,
    data: Vec<Option<T>>,
}

impl<T: Default + Copy> Matrix<T> {
    pub fn new(width: usize, height: usize) -> Result<Matrix<T>> {
        let size = width
            .checked_mul(height)
            .with_context(|| Error::invalid_argument(format!("Matrix::new: width * height overflow ({width} x {height})")))?;
        Ok(Self {
            width,
            height,
            data: vec![None; size],
        })
    }

    fn offset(x: usize, y: usize, width: usize) -> usize {
        y * width + x
    }

    pub fn get(&self, x: usize, y: usize) -> Option<T> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.data.get(Self::offset(x, y, self.width)).copied().flatten()
    }

    pub fn set(&mut self, x: usize, y: usize, value: T) -> Result<T> {
        ensure!(
            x < self.width && y < self.height,
            Error::invalid_argument(format!(
                    "set: coordinates ({x}, {y}) outside {}x{} matrix",
                    self.width, self.height
                ))
        );
        let offset = Self::offset(x, y, self.width);
        self.data[offset] = Some(value);
        Ok(value)
    }
}
