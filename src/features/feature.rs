use noodles::core::Position;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Feature {
    pub start: Position,
    pub end: Position,
}

impl Feature {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}
