use std::sync::Arc;

use crate::common::ECIStringBuilder;

#[derive(Debug, Default)]
pub struct DecoderResult {
    content: ECIStringBuilder,
    error: Option<Arc<anyhow::Error>>,
}

impl DecoderResult {
    pub fn with_eci_string_builder(src: ECIStringBuilder) -> Self {
        DecoderResult {
            content: src,
            error: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.content.symbology.code != 0 && self.error.is_none()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.content.into_bytes()
    }

    pub fn with_error(mut self, error: Option<anyhow::Error>) -> DecoderResult {
        self.error = error.map(Arc::new);
        self
    }
}
