use std::{
    collections::{HashMap, HashSet},
    io::{self, Read},
};

use atlas_core::collections::IntervalTree;
use noodles::{bam, core::Position, gff::record_buf::strand};
use tracing::warn;

use super::{
    match_intervals::MatchIntervals, segmented_reads::SegmentedReads,
    specification::StrandSpecification, Entry, Filter, IntervalTrees,
};

pub(super) enum Event<'f> {
    Hit(&'f str),
    Miss,
    Ambiguous,
    LowQuality,
    Unmapped,
    Nonunique,
    Skip,
}

pub type Counts<'f> = HashMap<&'f str, u64>;

#[derive(Default)]
pub struct Context<'f> {
    pub hits: Counts<'f>,
    pub miss: u64,
    pub ambiguous: u64,
    pub low_quality: u64,
    pub unmapped: u64,
    pub nonunique: u64,
}

impl<'f> Context<'f> {
    fn add_event(&mut self, event: Event<'f>) {
        match event {
            Event::Hit(name) => {
                let count = self.hits.entry(name).or_insert(0);
                *count += 1;
            }
            Event::Miss => self.miss += 1,
            Event::Ambiguous => self.ambiguous += 1,
            Event::LowQuality => self.low_quality += 1,
            Event::Unmapped => self.unmapped += 1,
            Event::Nonunique => self.nonunique += 1,
            Event::Skip => {}
        }
    }
}

pub(super) fn count_single_records<'f, R>(
    interval_trees: &IntervalTrees<'f>,
    filter: &'f Filter,
    strand_specification: StrandSpecification,
    mut reader: bam::io::Reader<R>,
) -> io::Result<Context<'f>>
where
    R: Read,
{
    let mut ctx = Context::default();
    let mut record = bam::Record::default();

    while reader.read_record(&mut record)? != 0 {
        let event = count_single_record(interval_trees, filter, strand_specification, &record)?;
        ctx.add_event(event);
    }

    Ok(ctx)
}

fn count_single_record<'f>(
    interval_trees: &IntervalTrees<'f>,
    filter: &'f Filter,
    strand_specification: StrandSpecification,
    record: &bam::Record,
) -> io::Result<Event<'f>> {
    if let Some(event) = filter.filter(record)? {
        return Ok(event);
    }

    let mut intersections = HashSet::new();

    let is_reverse_complemented = resolve_is_reverse_complemented(
        record.flags().is_reverse_complemented(),
        strand_specification,
    );

    if let Some(event) = count_record(
        interval_trees,
        strand_specification,
        is_reverse_complemented,
        record,
        &mut intersections,
    )? {
        return Ok(event);
    }

    Ok(resolve_intersections(&intersections))
}

pub(super) fn count_segmented_records<'f, R>(
    interval_trees: &IntervalTrees<'f>,
    filter: &'f Filter,
    strand_specification: StrandSpecification,
    reader: bam::io::Reader<R>,
) -> io::Result<Context<'f>>
where
    R: Read,
{
    let mut ctx = Context::default();
    let mut reads = SegmentedReads::new(reader);

    while let Some((r1, r2)) = reads.try_next()? {
        let event =
            count_segmented_records_inner(interval_trees, filter, strand_specification, &r1, &r2)?;

        ctx.add_event(event);
    }

    let unmatched_records = reads.unmatched_records();
    let unmatched_record_count = unmatched_records.len();

    if unmatched_record_count > 0 {
        warn!(unmatched_record_count, "found unmatched records");

        for record in unmatched_records {
            let event = count_single_record(interval_trees, filter, strand_specification, &record)?;
            ctx.add_event(event);
        }
    }

    Ok(ctx)
}

fn count_segmented_records_inner<'f>(
    interval_trees: &IntervalTrees<'f>,
    filter: &'f Filter,
    strand_specification: StrandSpecification,
    r1: &bam::Record,
    r2: &bam::Record,
) -> io::Result<Event<'f>> {
    if let Some(event) = filter.filter_segments(r1, r2)? {
        return Ok(event);
    }

    let mut intersections = HashSet::new();

    let r1_is_reverse_complemented =
        resolve_is_reverse_complemented(r1.flags().is_reverse_complemented(), strand_specification);

    if let Some(event) = count_record(
        interval_trees,
        strand_specification,
        r1_is_reverse_complemented,
        r1,
        &mut intersections,
    )? {
        return Ok(event);
    }

    let r2_is_reverse_complemented = !resolve_is_reverse_complemented(
        r2.flags().is_reverse_complemented(),
        strand_specification,
    );

    if let Some(event) = count_record(
        interval_trees,
        strand_specification,
        r2_is_reverse_complemented,
        r2,
        &mut intersections,
    )? {
        return Ok(event);
    }

    Ok(resolve_intersections(&intersections))
}

fn count_record<'f>(
    interval_trees: &IntervalTrees<'f>,
    strand_specification: StrandSpecification,
    is_reverse_complemented: bool,
    record: &bam::Record,
    intersections: &mut HashSet<&'f str>,
) -> io::Result<Option<Event<'f>>> {
    let reference_sequence_id = record
        .reference_sequence_id()
        .transpose()?
        .expect("missing reference sequence ID");

    let Some(interval_tree) = interval_trees.get(reference_sequence_id) else {
        return Ok(Some(Event::Miss));
    };

    let cigar = record.cigar();
    let mut ops = cigar.iter();

    let alignment_start = record
        .alignment_start()
        .transpose()?
        .expect("missing alignment start");

    let intervals = MatchIntervals::new(&mut ops, alignment_start);

    intersect(
        intersections,
        interval_tree,
        intervals,
        strand_specification,
        is_reverse_complemented,
    )?;

    Ok(None)
}

fn intersect<'f>(
    intersections: &mut HashSet<&'f str>,
    interval_tree: &IntervalTree<Position, Entry<'f>>,
    intervals: MatchIntervals<'_>,
    strand_specification: StrandSpecification,
    is_reverse_complemented: bool,
) -> io::Result<()> {
    use noodles::gff::record_buf::Strand;

    for result in intervals {
        let interval = result?;

        for (_, (name, strand)) in interval_tree.find(interval) {
            if strand_specification == StrandSpecification::None
                || (*strand == Strand::Reverse && is_reverse_complemented)
                || (*strand == Strand::Forward && !is_reverse_complemented)
            {
                intersections.insert(*name);
            }
        }
    }

    Ok(())
}

fn resolve_intersections<'f>(intersections: &HashSet<&'f str>) -> Event<'f> {
    if intersections.is_empty() {
        Event::Miss
    } else if intersections.len() == 1 {
        // SAFETY: `intersections` is non-empty.
        let name = intersections.iter().next().unwrap();
        Event::Hit(name)
    } else {
        Event::Ambiguous
    }
}

fn resolve_is_reverse_complemented(
    is_reverse_complemented: bool,
    strand_specification: StrandSpecification,
) -> bool {
    if strand_specification == StrandSpecification::Reverse {
        !is_reverse_complemented
    } else {
        is_reverse_complemented
    }
}
