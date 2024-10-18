use noodles::core::Position;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Feature {
    pub reference_sequence_name: String,
    pub start: Position,
    pub end: Position,
}

impl<'f> Feature {
    pub fn new<N>(reference_sequence_name: N, start: Position, end: Position) -> Self
    where
        N: Into<String>,
    {
        Self {
            reference_sequence_name: reference_sequence_name.into(),
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
        let feature = Feature::new("sq0", Position::try_from(5)?, Position::try_from(8)?);
        assert_eq!(feature.length(), 4);
        Ok(())
    }
}
