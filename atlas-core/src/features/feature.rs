use noodles::core::Position;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Feature {
    pub reference_sequence_id: usize,
    pub start: Position,
    pub end: Position,
}

impl Feature {
    pub fn new(reference_sequence_id: usize, start: Position, end: Position) -> Self {
        Self {
            reference_sequence_id,
            start,
            end,
        }
    }

    pub fn length(&self) -> usize {
        usize::from(self.end) - usize::from(self.start) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_length() -> Result<(), noodles::core::position::TryFromIntError> {
        let feature = Feature::new(0, Position::try_from(5)?, Position::try_from(8)?);
        assert_eq!(feature.length(), 4);
        Ok(())
    }
}
