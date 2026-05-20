use crate::qrcode::ErrorCorrectionLevel;

impl ErrorCorrectionLevel {
    pub fn eclevel_from_bits_signed(bits: i8) -> Self {
        let level_for_bits: [ErrorCorrectionLevel; 4] = [
            ErrorCorrectionLevel::M,
            ErrorCorrectionLevel::L,
            ErrorCorrectionLevel::H,
            ErrorCorrectionLevel::Q,
        ];
        level_for_bits[(bits as u8 & 0x03) as usize]
    }

    pub fn eclevel_from_bits(bits: u8) -> Self {
        Self::eclevel_from_bits_signed(bits as i8)
    }
}
