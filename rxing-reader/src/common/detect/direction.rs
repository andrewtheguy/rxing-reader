#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    pub const fn sign(self) -> i32 {
        match self {
            Self::Left => -1,
            Self::Right => 1,
        }
    }
}

impl From<Direction> for i32 {
    fn from(value: Direction) -> Self {
        value.sign()
    }
}
