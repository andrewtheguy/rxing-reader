use std::fmt::Display;

use crate::Error;

use super::CharacterSet;

const MAX_ECI_ASSIGNMENT_VALUE: u32 = 999_999;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Eci {
    Unknown,
    Cp437,
    ISO8859_1,
    ISO8859_2,
    ISO8859_3,
    ISO8859_4,
    ISO8859_5,
    ISO8859_6,
    ISO8859_7,
    ISO8859_8,
    ISO8859_9,
    ISO8859_10,
    ISO8859_11,
    ISO8859_13,
    ISO8859_14,
    ISO8859_15,
    ISO8859_16,
    ShiftJis,
    Cp1250,
    Cp1251,
    Cp1252,
    Cp1256,
    UTF16BE,
    UTF8,
    Ascii,
    Big5,
    GB2312,
    EucKr,
    GB18030,
    UTF16LE,
    UTF32BE,
    UTF32LE,
    Iso646Inv,
    Binary,
}

impl Eci {
    pub const fn assignment_number(self) -> Option<u32> {
        match self {
            Eci::Unknown => None,
            Eci::Cp437 => Some(2),
            Eci::ISO8859_1 => Some(3),
            Eci::ISO8859_2 => Some(4),
            Eci::ISO8859_3 => Some(5),
            Eci::ISO8859_4 => Some(6),
            Eci::ISO8859_5 => Some(7),
            Eci::ISO8859_6 => Some(8),
            Eci::ISO8859_7 => Some(9),
            Eci::ISO8859_8 => Some(10),
            Eci::ISO8859_9 => Some(11),
            Eci::ISO8859_10 => Some(12),
            Eci::ISO8859_11 => Some(13),
            Eci::ISO8859_13 => Some(15),
            Eci::ISO8859_14 => Some(16),
            Eci::ISO8859_15 => Some(17),
            Eci::ISO8859_16 => Some(18),
            Eci::ShiftJis => Some(20),
            Eci::Cp1250 => Some(21),
            Eci::Cp1251 => Some(22),
            Eci::Cp1252 => Some(23),
            Eci::Cp1256 => Some(24),
            Eci::UTF16BE => Some(25),
            Eci::UTF8 => Some(26),
            Eci::Ascii => Some(27),
            Eci::Big5 => Some(28),
            Eci::GB2312 => Some(29),
            Eci::EucKr => Some(30),
            Eci::GB18030 => Some(32),
            Eci::UTF16LE => Some(33),
            Eci::UTF32BE => Some(34),
            Eci::UTF32LE => Some(35),
            Eci::Iso646Inv => Some(170),
            Eci::Binary => Some(899),
        }
    }
}

impl TryFrom<u32> for Eci {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value > MAX_ECI_ASSIGNMENT_VALUE {
            return Err(Error::InvalidArgument {
                message: format!(
                    "ECI value {value} exceeds maximum assignment value {MAX_ECI_ASSIGNMENT_VALUE}"
                )
                .into(),
            }
            .into());
        }

        Ok(match value {
            0 | 2 => Eci::Cp437,
            1 | 3 => Eci::ISO8859_1,
            4 => Eci::ISO8859_2,
            5 => Eci::ISO8859_3,
            6 => Eci::ISO8859_4,
            7 => Eci::ISO8859_5,
            8 => Eci::ISO8859_6,
            9 => Eci::ISO8859_7,
            10 => Eci::ISO8859_8,
            11 => Eci::ISO8859_9,
            12 => Eci::ISO8859_10,
            13 => Eci::ISO8859_11,
            15 => Eci::ISO8859_13,
            16 => Eci::ISO8859_14,
            17 => Eci::ISO8859_15,
            18 => Eci::ISO8859_16,
            20 => Eci::ShiftJis,
            21 => Eci::Cp1250,
            22 => Eci::Cp1251,
            23 => Eci::Cp1252,
            24 => Eci::Cp1256,
            25 => Eci::UTF16BE,
            26 => Eci::UTF8,
            27 => Eci::Ascii,
            28 => Eci::Big5,
            29 => Eci::GB2312,
            30 => Eci::EucKr,
            32 => Eci::GB18030,
            33 => Eci::UTF16LE,
            34 => Eci::UTF32BE,
            35 => Eci::UTF32LE,
            170 => Eci::Iso646Inv,
            899 => Eci::Binary,
            _ => Eci::Unknown,
        })
    }
}

impl From<CharacterSet> for Eci {
    fn from(value: CharacterSet) -> Self {
        match value {
            CharacterSet::Cp437 => Eci::Cp437,
            CharacterSet::ISO8859_1 => Eci::ISO8859_1,
            CharacterSet::ISO8859_2 => Eci::ISO8859_2,
            CharacterSet::ISO8859_3 => Eci::ISO8859_3,
            CharacterSet::ISO8859_4 => Eci::ISO8859_4,
            CharacterSet::ISO8859_5 => Eci::ISO8859_5,
            CharacterSet::ISO8859_7 => Eci::ISO8859_7,
            CharacterSet::ISO8859_9 => Eci::ISO8859_9,
            CharacterSet::ISO8859_13 => Eci::ISO8859_13,
            CharacterSet::ISO8859_15 => Eci::ISO8859_15,
            CharacterSet::ISO8859_16 => Eci::ISO8859_16,
            CharacterSet::ShiftJis => Eci::ShiftJis,
            CharacterSet::Cp1250 => Eci::Cp1250,
            CharacterSet::Cp1251 => Eci::Cp1251,
            CharacterSet::Cp1252 => Eci::Cp1252,
            CharacterSet::Cp1256 => Eci::Cp1256,
            CharacterSet::UTF16BE => Eci::UTF16BE,
            CharacterSet::UTF8 => Eci::UTF8,
            CharacterSet::Ascii => Eci::Ascii,
            CharacterSet::Big5 => Eci::Big5,
            CharacterSet::GB2312 => Eci::GB2312,
            CharacterSet::GB18030 => Eci::GB18030,
            CharacterSet::EucKr => Eci::EucKr,
            CharacterSet::UTF16LE => Eci::UTF16LE,
            CharacterSet::UTF32BE => Eci::UTF32BE,
            CharacterSet::UTF32LE => Eci::UTF32LE,
            CharacterSet::Binary => Eci::Binary,
            CharacterSet::ISO8859_6 => Eci::ISO8859_6,
            CharacterSet::ISO8859_8 => Eci::ISO8859_8,
            CharacterSet::ISO8859_10 => Eci::ISO8859_10,
            CharacterSet::ISO8859_11 => Eci::ISO8859_11,
            CharacterSet::ISO8859_14 => Eci::ISO8859_14,
            _ => Eci::Unknown,
        }
    }
}

impl From<Eci> for CharacterSet {
    fn from(value: Eci) -> Self {
        match value {
            Eci::Cp437 => CharacterSet::Cp437,
            Eci::ISO8859_1 => CharacterSet::ISO8859_1,
            Eci::ISO8859_2 => CharacterSet::ISO8859_2,
            Eci::ISO8859_3 => CharacterSet::ISO8859_3,
            Eci::ISO8859_4 => CharacterSet::ISO8859_4,
            Eci::ISO8859_5 => CharacterSet::ISO8859_5,
            Eci::ISO8859_6 => CharacterSet::ISO8859_6,
            Eci::ISO8859_7 => CharacterSet::ISO8859_7,
            Eci::ISO8859_8 => CharacterSet::ISO8859_8,
            Eci::ISO8859_9 => CharacterSet::ISO8859_9,
            Eci::ISO8859_10 => CharacterSet::ISO8859_10,
            Eci::ISO8859_11 => CharacterSet::ISO8859_11,
            Eci::ISO8859_13 => CharacterSet::ISO8859_13,
            Eci::ISO8859_14 => CharacterSet::ISO8859_14,
            Eci::ISO8859_15 => CharacterSet::ISO8859_15,
            Eci::ISO8859_16 => CharacterSet::ISO8859_16,
            Eci::ShiftJis => CharacterSet::ShiftJis,
            Eci::Cp1250 => CharacterSet::Cp1250,
            Eci::Cp1251 => CharacterSet::Cp1251,
            Eci::Cp1252 => CharacterSet::Cp1252,
            Eci::Cp1256 => CharacterSet::Cp1256,
            Eci::UTF16BE => CharacterSet::UTF16BE,
            Eci::UTF8 => CharacterSet::UTF8,
            Eci::Ascii => CharacterSet::Ascii,
            Eci::Big5 => CharacterSet::Big5,
            Eci::GB2312 => CharacterSet::GB2312,
            Eci::EucKr => CharacterSet::EucKr,
            Eci::GB18030 => CharacterSet::GB18030,
            Eci::UTF16LE => CharacterSet::UTF16LE,
            Eci::UTF32BE => CharacterSet::UTF32BE,
            Eci::UTF32LE => CharacterSet::UTF32LE,
            Eci::Iso646Inv => CharacterSet::Ascii,
            Eci::Binary => CharacterSet::Binary,
            _ => CharacterSet::Unknown,
        }
    }
}

impl Display for Eci {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.assignment_number() {
            Some(value) => write!(f, "{value}"),
            None => f.write_str("unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Eci, MAX_ECI_ASSIGNMENT_VALUE};

    #[test]
    fn try_from_u32_rejects_values_outside_eci_range() {
        assert_eq!(Eci::try_from(26_u32).unwrap(), Eci::UTF8);
        assert!(Eci::try_from(MAX_ECI_ASSIGNMENT_VALUE + 1).is_err());
    }
}
