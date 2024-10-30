use std::{io, ops::RangeInclusive};

use noodles::{
    core::Position,
    sam::alignment::record::cigar::{op::Kind, Op},
};

pub struct MatchIntervals<'r> {
    ops: &'r mut dyn Iterator<Item = io::Result<Op>>,
    prev_alignment_start: Position,
}

impl<'r> MatchIntervals<'r> {
    pub fn new(
        ops: &'r mut dyn Iterator<Item = io::Result<Op>>,
        initial_alignment_start: Position,
    ) -> Self {
        Self {
            ops,
            prev_alignment_start: initial_alignment_start,
        }
    }
}

impl<'r> Iterator for MatchIntervals<'r> {
    type Item = io::Result<RangeInclusive<Position>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let op = match self.ops.next()? {
                Ok(op) => op,
                Err(e) => return Some(Err(e)),
            };

            let len = op.len();

            assert!(len > 0);

            match op.kind() {
                Kind::Match | Kind::SequenceMatch | Kind::SequenceMismatch => {
                    let start = self.prev_alignment_start;

                    let end = start
                        // SAFETY: `len` is non-zero.
                        .checked_add(len - 1)
                        .expect("attempt to add with overflow");

                    self.prev_alignment_start = self
                        .prev_alignment_start
                        .checked_add(len)
                        .expect("attempt to add with overflow");

                    return Some(Ok(start..=end));
                }
                Kind::Deletion | Kind::Skip => {
                    self.prev_alignment_start = self
                        .prev_alignment_start
                        .checked_add(len)
                        .expect("attempt to add with overflow");
                }
                _ => continue,
            }
        }
    }
}
