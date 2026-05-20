use crate::common::BitMatrix;

pub(crate) struct QRCodeDetectorResult {
    bits: BitMatrix,
}

impl QRCodeDetectorResult {
    pub(crate) fn new(bits: BitMatrix) -> Self {
        Self { bits }
    }

    pub(crate) fn bits(&self) -> &BitMatrix {
        &self.bits
    }
}