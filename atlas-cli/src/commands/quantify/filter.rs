use std::io;

use noodles::bam;

use super::count::Event;

pub(super) struct Filter;

impl Filter {
    pub(super) fn filter(&self, record: &bam::Record) -> io::Result<Option<Event<'_>>> {
        let flags = record.flags();

        if flags.is_unmapped() {
            return Ok(Some(Event::Unmapped));
        }

        Ok(None)
    }
}
