use std::sync::Arc;

use crate::common::ECIStringBuilder;
use crate::qrcode::{ErrorCorrectionLevel, Mode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuredAppendInfo {
    pub index: u8,
    pub count: u8,
    pub parity: u8,
}

#[derive(Debug, Default)]
pub struct DecoderResult {
    content: ECIStringBuilder,
    error: Option<Arc<anyhow::Error>>,
    version: u32,
    error_correction_level: Option<ErrorCorrectionLevel>,
    mask: u8,
    modes: Vec<Mode>,
    structured_append: Option<StructuredAppendInfo>,
}

impl DecoderResult {
    pub fn with_eci_string_builder(src: ECIStringBuilder) -> Self {
        DecoderResult {
            content: src,
            error: None,
            version: 0,
            error_correction_level: None,
            mask: 0,
            modes: Vec::new(),
            structured_append: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.content.symbology.code != 0 && self.error.is_none()
    }

    pub fn with_error(mut self, error: Option<anyhow::Error>) -> DecoderResult {
        self.error = error.map(Arc::new);
        self
    }

    pub fn with_format(
        mut self,
        version: u32,
        error_correction_level: ErrorCorrectionLevel,
        mask: u8,
    ) -> DecoderResult {
        self.version = version;
        self.error_correction_level = Some(error_correction_level);
        self.mask = mask;
        self
    }

    pub fn set_modes(&mut self, modes: Vec<Mode>) {
        self.modes = modes;
    }

    pub fn set_structured_append(&mut self, info: StructuredAppendInfo) {
        self.structured_append = Some(info);
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn error_correction_level(&self) -> Option<ErrorCorrectionLevel> {
        self.error_correction_level
    }

    pub fn mask(&self) -> u8 {
        self.mask
    }

    pub fn modes(&self) -> &[Mode] {
        &self.modes
    }

    pub fn structured_append(&self) -> Option<StructuredAppendInfo> {
        self.structured_append
    }

    pub fn into_content(self) -> ECIStringBuilder {
        self.content
    }
}
