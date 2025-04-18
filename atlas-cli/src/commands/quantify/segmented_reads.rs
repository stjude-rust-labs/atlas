use std::{
    collections::HashMap,
    io::{self, Read},
};

use noodles::{bam, sam::alignment::record::Flags};
use thiserror::Error;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum SegmentPosition {
    First,
    Last,
}

impl SegmentPosition {
    fn mate(self) -> Self {
        match self {
            Self::First => Self::Last,
            Self::Last => Self::First,
        }
    }
}

#[derive(Debug, Error)]
enum TryFromFlagsError {
    #[error("ambiguous segment position")]
    Ambiguous,
    #[error("missing segment position")]
    Missing,
}

impl TryFrom<Flags> for SegmentPosition {
    type Error = TryFromFlagsError;

    fn try_from(flags: Flags) -> Result<Self, Self::Error> {
        const BOTH: Flags = Flags::FIRST_SEGMENT.union(Flags::LAST_SEGMENT);

        if flags.contains(BOTH) {
            Err(TryFromFlagsError::Ambiguous)
        } else if flags.is_first_segment() {
            Ok(Self::First)
        } else if flags.is_last_segment() {
            Ok(Self::Last)
        } else {
            Err(TryFromFlagsError::Missing)
        }
    }
}

pub(super) struct SegmentedReads<R> {
    reader: bam::io::Reader<R>,
    cache: HashMap<Vec<u8>, Vec<bam::Record>>,
}

impl<R> SegmentedReads<R>
where
    R: Read,
{
    pub(super) fn new(reader: bam::io::Reader<R>) -> Self {
        Self {
            reader,
            cache: HashMap::new(),
        }
    }

    pub(super) fn unmatched_records(self) -> impl Iterator<Item = bam::Record> {
        self.cache.into_values().flatten()
    }

    fn try_next(&mut self) -> io::Result<Option<(bam::Record, bam::Record)>> {
        use std::collections::hash_map::Entry;

        loop {
            let mut record = bam::Record::default();

            if self.reader.read_record(&mut record)? == 0 {
                return Ok(None);
            }

            let flags = record.flags();

            if !is_primary(flags) {
                continue;
            }

            let name = record.name().unwrap();

            match self.cache.entry(name.to_vec()) {
                Entry::Occupied(mut entry) => {
                    let records = entry.get_mut();

                    let Some(i) = find_mate(records, &record)? else {
                        records.push(record);
                        continue;
                    };

                    let mate = records.swap_remove(i);

                    if records.is_empty() {
                        entry.remove();
                    }

                    let segment_position = SegmentPosition::try_from(flags)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                    return match segment_position {
                        SegmentPosition::First => Ok(Some((record, mate))),
                        SegmentPosition::Last => Ok(Some((mate, record))),
                    };
                }
                Entry::Vacant(entry) => {
                    entry.insert(vec![record]);
                }
            }
        }
    }
}

impl<R> Iterator for SegmentedReads<R>
where
    R: Read,
{
    type Item = io::Result<(bam::Record, bam::Record)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.try_next() {
            Ok(Some(segments)) => Some(Ok(segments)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

fn is_primary(flags: Flags) -> bool {
    const FILTERS: Flags = Flags::SECONDARY.union(Flags::SUPPLEMENTARY);
    !flags.intersects(FILTERS)
}

fn find_mate(records: &[bam::Record], record: &bam::Record) -> io::Result<Option<usize>> {
    for (i, mate) in records.iter().enumerate() {
        if is_mate(record, mate)? {
            return Ok(Some(i));
        }
    }

    Ok(None)
}

fn is_mate(a: &bam::Record, b: &bam::Record) -> io::Result<bool> {
    let a_fields = (
        SegmentPosition::try_from(a.flags())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        a.reference_sequence_id().transpose()?,
        a.alignment_start().transpose()?,
        a.mate_reference_sequence_id().transpose()?,
        a.mate_alignment_start().transpose()?,
        a.template_length(),
    );

    let b_fields = (
        SegmentPosition::try_from(b.flags())
            .map(|position| position.mate())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        b.mate_reference_sequence_id().transpose()?,
        b.mate_alignment_start().transpose()?,
        b.reference_sequence_id().transpose()?,
        b.alignment_start().transpose()?,
        -b.template_length(),
    );

    Ok(a_fields == b_fields)
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use noodles::{
        core::Position,
        sam::{
            self,
            alignment::io::Write,
            header::record::value::{Map, map::ReferenceSequence},
        },
    };

    use super::*;

    fn encode_records() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let alignment_start = Position::try_from(8)?;
        let mate_alignment_start = Position::try_from(13)?;

        let header = sam::Header::builder()
            .add_reference_sequence(
                "sq0",
                Map::<ReferenceSequence>::new(NonZero::new(34).unwrap()),
            )
            .build();

        let records = [
            sam::alignment::RecordBuf::builder()
                .set_name("r0")
                .set_flags(Flags::SEGMENTED | Flags::FIRST_SEGMENT)
                .set_reference_sequence_id(0)
                .set_alignment_start(alignment_start)
                .set_mate_reference_sequence_id(0)
                .set_mate_alignment_start(mate_alignment_start)
                .set_template_length(6)
                .build(),
            sam::alignment::RecordBuf::builder()
                .set_name("r0")
                .set_flags(Flags::SEGMENTED | Flags::LAST_SEGMENT)
                .set_reference_sequence_id(0)
                .set_alignment_start(mate_alignment_start)
                .set_mate_reference_sequence_id(0)
                .set_mate_alignment_start(alignment_start)
                .set_template_length(-6)
                .build(),
            sam::alignment::RecordBuf::builder()
                .set_name("r1")
                .set_flags(Flags::SEGMENTED | Flags::FIRST_SEGMENT)
                .set_reference_sequence_id(0)
                .set_alignment_start(alignment_start)
                .set_mate_reference_sequence_id(0)
                .set_mate_alignment_start(mate_alignment_start)
                .set_template_length(9)
                .build(),
        ];

        let mut writer = bam::io::Writer::from(Vec::new());

        for record in &records {
            writer.write_alignment_record(&header, record)?;
        }

        Ok(writer.into_inner())
    }

    #[test]
    fn test_try_next() -> Result<(), Box<dyn std::error::Error>> {
        let src = encode_records()?;
        let reader = bam::io::Reader::from(&src[..]);
        let mut reads = SegmentedReads::new(reader);

        let (a, b) = reads.try_next()?.unwrap();

        assert_eq!(a.name(), b.name());

        assert!(a.flags().is_segmented());
        assert!(b.flags().is_segmented());

        assert!(a.flags().is_first_segment());
        assert!(b.flags().is_last_segment());

        assert_eq!(
            a.reference_sequence_id().transpose()?,
            b.mate_reference_sequence_id().transpose()?
        );

        assert_eq!(
            a.alignment_start().transpose()?,
            b.mate_alignment_start().transpose()?
        );

        assert_eq!(
            a.mate_reference_sequence_id().transpose()?,
            b.reference_sequence_id().transpose()?
        );

        assert_eq!(
            a.mate_alignment_start().transpose()?,
            b.alignment_start().transpose()?
        );

        assert_eq!(a.template_length(), -b.template_length());

        assert!(reads.try_next()?.is_none());

        Ok(())
    }

    #[test]
    fn test_is_primary() {
        assert!(is_primary(Flags::default()));
        assert!(!is_primary(Flags::SECONDARY));
        assert!(!is_primary(Flags::SUPPLEMENTARY));
        assert!(!is_primary(Flags::SECONDARY | Flags::SUPPLEMENTARY));
    }
}
