use std::io;

use noodles::{bam, sam::alignment::record::MappingQuality};

use super::count::Event;

pub(super) struct Filter {
    min_mapping_quality: MappingQuality,
}

impl Filter {
    pub(super) fn new(min_mapping_quality: MappingQuality) -> Self {
        Self {
            min_mapping_quality,
        }
    }

    pub(super) fn filter(&self, record: &bam::Record) -> io::Result<Option<Event<'_>>> {
        let flags = record.flags();

        if flags.is_unmapped() {
            return Ok(Some(Event::Unmapped));
        }

        if let Some(mapping_quality) = record.mapping_quality() {
            if mapping_quality < self.min_mapping_quality {
                return Ok(Some(Event::LowQuality));
            }
        }

        Ok(None)
    }
}
