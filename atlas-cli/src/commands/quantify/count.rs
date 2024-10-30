use std::{
    collections::{HashMap, HashSet},
    io::{self, Read},
};

use atlas_core::collections::IntervalTree;
use noodles::{bam, core::Position};

use super::{match_intervals::MatchIntervals, Entry, IntervalTrees};

#[expect(dead_code)]
enum Event<'f> {
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
    mut reader: bam::io::Reader<R>,
) -> io::Result<Context<'f>>
where
    R: Read,
{
    let mut ctx = Context::default();
    let mut record = bam::Record::default();

    while reader.read_record(&mut record)? != 0 {
        let event = count_single_record(interval_trees, &record)?;
        ctx.add_event(event);
    }

    Ok(ctx)
}

fn count_single_record<'f>(
    interval_trees: &IntervalTrees<'f>,
    record: &bam::Record,
) -> io::Result<Event<'f>> {
    let reference_sequence_id = record
        .reference_sequence_id()
        .transpose()?
        .expect("missing reference sequence ID");

    let Some(interval_tree) = interval_trees.get(reference_sequence_id) else {
        return Ok(Event::Miss);
    };

    let cigar = record.cigar();
    let mut ops = cigar.iter();

    let alignment_start = record
        .alignment_start()
        .transpose()?
        .expect("missing alignment start");

    let intervals = MatchIntervals::new(&mut ops, alignment_start);

    let intersections = intersect(interval_tree, intervals)?;

    if intersections.is_empty() {
        Ok(Event::Miss)
    } else if intersections.len() == 1 {
        // SAFETY: `intersections` is non-empty.
        let name = intersections.iter().next().unwrap();
        Ok(Event::Hit(name))
    } else {
        Ok(Event::Ambiguous)
    }
}

fn intersect<'f>(
    interval_tree: &IntervalTree<Position, Entry<'f>>,
    intervals: MatchIntervals<'_>,
) -> io::Result<HashSet<&'f str>> {
    let mut set = HashSet::new();

    for result in intervals {
        let interval = result?;

        for (_, (name, _)) in interval_tree.find(interval) {
            set.insert(*name);
        }
    }

    Ok(set)
}
