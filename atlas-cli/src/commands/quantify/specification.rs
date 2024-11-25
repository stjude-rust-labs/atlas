use std::io::{self, Read};

use atlas_core::collections::IntervalTree;
use noodles::{
    bam,
    core::Position,
    gff,
    sam::alignment::{record::Flags, Record as _},
};

use super::{Entry, IntervalTrees};

#[derive(Clone, Copy, Debug)]
pub(super) enum LibraryLayout {
    Single,
    Multiple,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StrandSpecification {
    None,
    Forward,
    Reverse,
}

#[derive(Default)]
struct Counts {
    segmented: u64,
    matches: u64,
    forward: u64,
    reverse: u64,
}

#[derive(Clone, Copy)]
enum Strand {
    Forward,
    Reverse,
}

impl From<Flags> for Strand {
    fn from(flags: Flags) -> Self {
        if flags.is_reverse_complemented() {
            Self::Reverse
        } else {
            Self::Forward
        }
    }
}

impl TryFrom<gff::record::Strand> for Strand {
    type Error = ();

    fn try_from(strand: gff::record::Strand) -> Result<Self, Self::Error> {
        match strand {
            gff::record::Strand::Forward => Ok(Self::Forward),
            gff::record::Strand::Reverse => Ok(Self::Reverse),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy)]
enum SegmentPosition {
    First,
    Last,
}

impl TryFrom<Flags> for SegmentPosition {
    type Error = ();

    fn try_from(flags: Flags) -> Result<Self, Self::Error> {
        if flags.is_first_segment() {
            Ok(Self::First)
        } else if flags.is_last_segment() {
            Ok(Self::Last)
        } else {
            // TODO: `flags.intersects(Flags::FIRST_SEGMENT | Flags::LAST_SEGMENT)`.
            Err(())
        }
    }
}

pub(super) fn detect<R>(
    reader: &mut bam::io::Reader<R>,
    interval_trees: &IntervalTrees<'_>,
) -> io::Result<(LibraryLayout, StrandSpecification)>
where
    R: Read,
{
    const MAX_RECORD_COUNT: usize = 1 << 19;
    const FILTERS: Flags = Flags::UNMAPPED
        .union(Flags::SECONDARY)
        .union(Flags::SUPPLEMENTARY);
    const STRANDEDNESS_THRESHOLD: f64 = 0.75;

    let mut record = bam::Record::default();
    let mut n = 0;

    let mut counts = Counts::default();

    while reader.read_record(&mut record)? != 0 {
        if n >= MAX_RECORD_COUNT {
            break;
        }

        let flags = record.flags();

        if flags.intersects(FILTERS) {
            continue;
        }

        let reference_sequence_id = record
            .reference_sequence_id()
            .transpose()?
            .expect("missing reference sequence ID");

        let Some(tree) = interval_trees.get(reference_sequence_id) else {
            continue;
        };

        let alignment_start = record
            .alignment_start()
            .transpose()?
            .expect("missing alignment start");

        let alignment_end = record
            .alignment_end()
            .transpose()?
            .expect("missing alignment end");

        if flags.is_segmented() {
            counts.segmented += 1;
            count_segmented_record(&mut counts, tree, flags, alignment_start, alignment_end)?;
        } else {
            count_single_record(&mut counts, tree, flags, alignment_start, alignment_end);
        }

        n += 1;
    }

    let library_layout = if counts.segmented > 0 {
        LibraryLayout::Multiple
    } else {
        LibraryLayout::Single
    };

    if counts.matches == 0 {
        return Ok((library_layout, StrandSpecification::None));
    }

    // TODO: check f64 range
    let matches = counts.matches as f64;
    let forward_pct = (counts.forward as f64) / matches;
    let reverse_pct = (counts.reverse as f64) / matches;

    let strand_specification = if forward_pct > STRANDEDNESS_THRESHOLD {
        StrandSpecification::Forward
    } else if reverse_pct > STRANDEDNESS_THRESHOLD {
        StrandSpecification::Reverse
    } else {
        StrandSpecification::None
    };

    Ok((library_layout, strand_specification))
}

fn count_single_record(
    counts: &mut Counts,
    tree: &IntervalTree<Position, Entry<'_>>,
    flags: Flags,
    alignment_start: Position,
    alignment_end: Position,
) {
    let interval = alignment_start..=alignment_end;
    let record_strand = Strand::from(flags);

    for (_, (_, strand)) in tree.find(interval) {
        let Ok(feature_strand) = Strand::try_from(*strand) else {
            continue;
        };

        match (record_strand, feature_strand) {
            (Strand::Forward, Strand::Forward) | (Strand::Reverse, Strand::Reverse) => {
                counts.forward += 1
            }
            (Strand::Forward, Strand::Reverse) | (Strand::Reverse, Strand::Forward) => {
                counts.forward += 1
            }
        }

        counts.matches += 1;
    }
}

fn count_segmented_record(
    counts: &mut Counts,
    tree: &IntervalTree<Position, Entry<'_>>,
    flags: Flags,
    alignment_start: Position,
    alignment_end: Position,
) -> io::Result<()> {
    let interval = alignment_start..=alignment_end;

    let segment_position = SegmentPosition::try_from(flags)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid segment position"))?;
    let record_strand = Strand::from(flags);

    for (_, (_, strand)) in tree.find(interval) {
        let Ok(feature_strand) = Strand::try_from(*strand) else {
            continue;
        };

        match (segment_position, record_strand, feature_strand) {
            (SegmentPosition::First, Strand::Forward, Strand::Forward)
            | (SegmentPosition::First, Strand::Reverse, Strand::Reverse)
            | (SegmentPosition::Last, Strand::Forward, Strand::Reverse)
            | (SegmentPosition::Last, Strand::Reverse, Strand::Forward) => {
                counts.forward += 1;
            }
            (SegmentPosition::First, Strand::Forward, Strand::Reverse)
            | (SegmentPosition::First, Strand::Reverse, Strand::Forward)
            | (SegmentPosition::Last, Strand::Forward, Strand::Forward)
            | (SegmentPosition::Last, Strand::Reverse, Strand::Reverse) => {
                counts.reverse += 1;
            }
        }
    }

    Ok(())
}
