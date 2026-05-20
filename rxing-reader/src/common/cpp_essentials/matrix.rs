use crate::Error;
use anyhow::Result;

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
            .ok_or_else(|| Error::InvalidArgument {
                message: format!("Matrix::new: width * height overflow ({width} x {height})").into(),
            })?;
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
            None
        } else if let Some(Some(d)) = self.data.get(Self::offset(x, y, self.width)) {
            Some(*d)
        } else {
            None
        }
    }

    pub fn set(&mut self, x: usize, y: usize, value: T) -> Result<T> {
        if x >= self.width || y >= self.height {
            return Err(Error::InvalidArgument {
                message: format!(
                    "set: coordinates ({x}, {y}) outside {}x{} matrix",
                    self.width, self.height
                ).into(),
            }
            .into());
        }
        let offset = Self::offset(x, y, self.width);
        self.data[offset] = Some(value);
        Ok(value)
    }
}
