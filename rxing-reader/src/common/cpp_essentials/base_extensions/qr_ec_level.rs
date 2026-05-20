use crate::qrcode::common::ErrorCorrectionLevel;

impl ErrorCorrectionLevel {
    pub fn eclevel_from_bits_signed(bits: i8, is_micro: bool) -> Self {
        if is_micro {
            let level_for_bits: [ErrorCorrectionLevel; 8] = [
                ErrorCorrectionLevel::L,
                ErrorCorrectionLevel::L,
                ErrorCorrectionLevel::M,
                ErrorCorrectionLevel::L,
                ErrorCorrectionLevel::M,
                ErrorCorrectionLevel::L,
                ErrorCorrectionLevel::M,
                ErrorCorrectionLevel::Q,
            ];
            return level_for_bits[(bits as u8 & 0x07) as usize];
        }
        let level_for_bits: [ErrorCorrectionLevel; 4] = [
            ErrorCorrectionLevel::M,
            ErrorCorrectionLevel::L,
            ErrorCorrectionLevel::H,
            ErrorCorrectionLevel::Q,
        ];
        level_for_bits[(bits as u8 & 0x03) as usize]
    }

    pub fn eclevel_from_bits(bits: u8, is_micro: bool) -> Self {
        Self::eclevel_from_bits_signed(bits as i8, is_micro)
    }
}
