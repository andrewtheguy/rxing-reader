use crate::common::Result;
use crate::{Exceptions, Point};

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Matrix<T: Default + Clone + Copy> {
    width: usize,
    height: usize,
    data: Vec<Option<T>>,
}

impl<T: Default + Clone + Copy> Matrix<T> {
    pub fn with_data(width: usize, height: usize, data: Vec<Option<T>>) -> Result<Matrix<T>> {
        let expected = width.checked_mul(height).ok_or_else(|| {
            Exceptions::illegal_argument_with("invalid size: width * height overflow")
        })?;
        if data.len() != expected {
            return Err(Exceptions::illegal_argument_with(
                "invalid size: data length does not match width * height",
            ));
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub fn new(width: usize, height: usize) -> Result<Matrix<T>> {
        let size = width.checked_mul(height).ok_or_else(|| {
            Exceptions::illegal_argument_with("invalid size: width * height is too big")
        })?;
        Ok(Self {
            width,
            height,
            data: vec![None; size],
        })
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    fn get_offset(x: usize, y: usize, width: usize) -> usize {
        y * width + x
    }

    pub fn get(&self, x: usize, y: usize) -> Option<T> {
        if x >= self.width || y >= self.height {
            None
        } else if let Some(Some(d)) = self.data.get(Self::get_offset(x, y, self.width)) {
            Some(*d)
        } else {
            None
        }
    }

    pub fn set(&mut self, x: usize, y: usize, value: T) -> Result<T> {
        if x >= self.width || y >= self.height {
            return Err(Exceptions::index_out_of_bounds_with(format!(
                "set: coordinates ({x}, {y}) outside {}x{} matrix",
                self.width, self.height
            )));
        }
        let offset = Self::get_offset(x, y, self.width);
        self.data[offset] = Some(value);
        Ok(value)
    }

    pub fn get_point(&self, p: Point) -> Option<T> {
        self.get(p.x as usize, p.y as usize)
    }

    pub fn set_point(&mut self, p: Point, value: T) -> Result<T> {
        if !p.x.is_finite() || !p.y.is_finite() {
            return Err(Exceptions::illegal_argument_with(format!(
                "set_point: non-finite coordinates ({}, {})",
                p.x, p.y
            )));
        }
        if p.x < 0.0 || p.y < 0.0 {
            return Err(Exceptions::index_out_of_bounds_with(format!(
                "set_point: negative coordinates ({}, {})",
                p.x, p.y
            )));
        }
        let x = f64::from(p.x);
        let y = f64::from(p.y);
        if x >= self.width as f64 || y >= self.height as f64 {
            return Err(Exceptions::index_out_of_bounds_with(format!(
                "set_point: coordinates ({}, {}) outside {}x{} matrix",
                p.x, p.y, self.width, self.height
            )));
        }
        if x > usize::MAX as f64 || y > usize::MAX as f64 {
            return Err(Exceptions::index_out_of_bounds_with(format!(
                "set_point: coordinates ({}, {}) cannot be represented as usize",
                p.x, p.y
            )));
        }
        self.set(p.x as usize, p.y as usize, value)
    }

    pub fn data(&self) -> &[Option<T>] {
        &self.data
    }

    pub fn clear_with(&mut self, value: T) {
        self.data.fill(Some(value))
    }

    pub fn clear(&mut self) {
        self.data.fill(None)
    }
}
