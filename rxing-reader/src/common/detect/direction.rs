#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Direction {
    Left = -1,
    Right = 1,
}

impl From<Direction> for i32 {
    fn from(value: Direction) -> Self {
        value as i32
    }
}
