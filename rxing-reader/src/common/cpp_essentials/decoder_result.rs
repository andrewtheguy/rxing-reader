use std::sync::Arc;

use crate::common::ECIStringBuilder;

use super::StructuredAppendInfo;

const SYMBOLOGY_MODIFIER_MODEL_1: u8 = b'0';

#[derive(Debug)]
pub struct DecoderResult<T>
where
    T: Copy + Default + Eq + PartialEq,
{
    content: ECIStringBuilder,
    ec_level: String,
    line_count: u32,     // = 0;
    version_number: u32, // = 0;
    structured_append: StructuredAppendInfo,
    is_mirrored: bool, // = false;
    reader_init: bool, // = false;
    error: Option<Arc<anyhow::Error>>,
    extra: T,
}

impl<T> Default for DecoderResult<T>
where
    T: Copy + Default + Eq + PartialEq,
{
    fn default() -> Self {
        Self {
            content: Default::default(),
            ec_level: Default::default(),
            line_count: 0,
            version_number: 0,
            structured_append: Default::default(),
            is_mirrored: false,
            reader_init: false,
            error: None,
            extra: Default::default(),
        }
    }
}

impl<T> DecoderResult<T>
where
    T: Copy + Default + Eq + PartialEq,
{
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_eci_string_builder(src: ECIStringBuilder) -> Self {
        DecoderResult::<T> {
            content: src,
            ..Default::default()
        }
    }

    pub fn is_valid(&self) -> bool {
        self.content.symbology.code != 0 && self.error.is_none()
    }

    pub fn content(&self) -> &ECIStringBuilder {
        &self.content
    }
}

impl<T> DecoderResult<T>
where
    T: Copy + Default + Eq + PartialEq,
{
    pub fn ec_level(&self) -> &str {
        &self.ec_level
    }
    pub fn set_ec_level(&mut self, ec_level: String) {
        self.ec_level = ec_level
    }
    pub fn with_ec_level(mut self, ec_level: String) -> DecoderResult<T> {
        self.set_ec_level(ec_level);
        self
    }

    pub fn line_count(&self) -> u32 {
        self.line_count
    }
    pub fn set_line_count(&mut self, lc: u32) {
        self.line_count = lc
    }
    pub fn with_line_count(mut self, lc: u32) -> DecoderResult<T> {
        self.set_line_count(lc);
        self
    }

    pub fn version_number(&self) -> u32 {
        self.version_number
    }
    pub fn set_version_number(&mut self, vn: u32) {
        self.version_number = vn
    }
    pub fn with_version_number(mut self, vn: u32) -> DecoderResult<T> {
        self.set_version_number(vn);
        self
    }

    pub fn structured_append(&self) -> &StructuredAppendInfo {
        &self.structured_append
    }
    pub fn set_structured_append(&mut self, sai: StructuredAppendInfo) {
        self.structured_append = sai
    }
    pub fn with_structured_append(mut self, sai: StructuredAppendInfo) -> DecoderResult<T> {
        self.set_structured_append(sai);
        self
    }

    pub fn is_mirrored(&self) -> bool {
        self.is_mirrored
    }
    pub fn set_is_mirrored(&mut self, is_mirrored: bool) {
        self.is_mirrored = is_mirrored
    }
    pub fn with_is_mirrored(mut self, is_mirrored: bool) -> DecoderResult<T> {
        self.set_is_mirrored(is_mirrored);
        self
    }

    pub fn reader_init(&self) -> bool {
        self.reader_init
    }
    pub fn set_reader_init(&mut self, reader_init: bool) {
        self.reader_init = reader_init
    }
    pub fn with_reader_init(mut self, reader_init: bool) -> DecoderResult<T> {
        self.set_reader_init(reader_init);
        self
    }

    pub fn extra(&self) -> T {
        self.extra
    }
    pub fn set_extra(&mut self, extra: T) {
        self.extra = extra
    }
    pub fn with_extra(mut self, extra: T) -> DecoderResult<T> {
        self.set_extra(extra);
        self
    }

    pub fn error(&self) -> Option<&anyhow::Error> {
        self.error.as_deref()
    }
    pub fn set_error(&mut self, error: Option<anyhow::Error>) {
        self.error = error.map(Arc::new)
    }
    pub fn with_error(mut self, error: Option<anyhow::Error>) -> DecoderResult<T> {
        self.set_error(error);
        self
    }

    pub fn with_is_model1(mut self, is_model_1: bool) -> DecoderResult<T> {
        if is_model_1 {
            self.content.symbology.modifier = SYMBOLOGY_MODIFIER_MODEL_1
        }
        self
    }
}

impl<T> DecoderResult<T>
where
    T: Copy + Default + Eq + PartialEq,
{
    pub fn text(&self) -> String {
        self.content.to_string()
    }

    pub fn symbology_identifier(&self) -> String {
        let s = self.content.symbology;
        if s.code > 0 {
            format!(
                "]{}{}",
                char::from(s.code),
                char::from(
                    s.modifier
                        + if self.content.has_eci {
                            s.eci_modifier_offset
                        } else {
                            0
                        }
                )
            )
        } else {
            String::default()
        }
    }
}
