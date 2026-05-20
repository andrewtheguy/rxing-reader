use crate::{
    Point,
    common::{BitMatrix, DetectorRXingResult},
};

pub struct QRCodeDetectorResult {
    bit_source: BitMatrix,
    result_points: Vec<Point>,
}

impl QRCodeDetectorResult {
    pub fn new(bit_source: BitMatrix, result_points: Vec<Point>) -> Self {
        Self {
            bit_source,
            result_points,
        }
    }
}

impl DetectorRXingResult for QRCodeDetectorResult {
    fn get_bits(&self) -> &crate::common::BitMatrix {
        &self.bit_source
    }

    fn get_points(&self) -> &[crate::Point] {
        &self.result_points
    }
}
