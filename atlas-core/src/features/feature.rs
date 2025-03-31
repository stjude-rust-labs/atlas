use noodles::{core::Position, gff::feature::record::Strand};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Feature {
    pub reference_sequence_id: usize,
    pub start: Position,
    pub end: Position,
    pub strand: Strand,
}

impl Feature {
    pub fn new(
        reference_sequence_id: usize,
        start: Position,
        end: Position,
        strand: Strand,
    ) -> Self {
        Self {
            reference_sequence_id,
            start,
            end,
            strand,
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
        let feature = Feature::new(
            0,
            Position::try_from(5)?,
            Position::try_from(8)?,
            Strand::Forward,
        );

        assert_eq!(feature.length(), 4);

        Ok(())
    }
}
