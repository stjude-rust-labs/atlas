use std::io;

use noodles::{
    bam,
    sam::alignment::record::{Flags, MappingQuality},
};

use super::count::Event;

const NONPRIMARY: Flags = Flags::SECONDARY.union(Flags::SUPPLEMENTARY);

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

        if flags.intersects(NONPRIMARY) {
            return Ok(Some(Event::Skip));
        }

        if !is_unique_record(record)? {
            return Ok(Some(Event::Nonunique));
        }

        if let Some(mapping_quality) = record.mapping_quality() {
            if mapping_quality < self.min_mapping_quality {
                return Ok(Some(Event::LowQuality));
            }
        }

        Ok(None)
    }

    pub(super) fn filter_segments(
        &self,
        r1: &bam::Record,
        r2: &bam::Record,
    ) -> io::Result<Option<Event<'_>>> {
        let f1 = r1.flags();
        let f2 = r2.flags();

        if f1.is_unmapped() && f2.is_unmapped() {
            return Ok(Some(Event::Unmapped));
        }

        if f1.intersects(NONPRIMARY) || f2.intersects(NONPRIMARY) {
            return Ok(Some(Event::Skip));
        }

        if !is_unique_record(r1)? || !is_unique_record(r2)? {
            return Ok(Some(Event::Nonunique));
        }

        if let Some(mapping_quality) = r1.mapping_quality() {
            if mapping_quality < self.min_mapping_quality {
                return Ok(Some(Event::LowQuality));
            }
        }

        if let Some(mapping_quality) = r2.mapping_quality() {
            if mapping_quality < self.min_mapping_quality {
                return Ok(Some(Event::LowQuality));
            }
        }

        Ok(None)
    }
}

fn is_unique_record(record: &bam::Record) -> io::Result<bool> {
    use noodles::sam::alignment::record::data::field::Tag;

    let data = record.data();

    let Some(value) = data.get(&Tag::ALIGNMENT_HIT_COUNT).transpose()? else {
        return Ok(false);
    };

    match value.as_int() {
        Some(n) => {
            // A unique record should have an alignment hit count (`NH`) of 1, but htseq-count
            // 0.12.3 seems to also allow this be 0 or negative.
            Ok(n <= 1)
        }
        None => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "invalid {:?} field value type: expected an integer, got {:?}",
                Tag::ALIGNMENT_HIT_COUNT,
                value.ty(),
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use noodles::sam::{
        self,
        alignment::{io::Write, record::data::field::Tag, record_buf::data::field::Value},
    };

    use super::*;

    #[test]
    fn test_is_unique_record() -> io::Result<()> {
        fn build_record(alignment_hit_count: Value) -> io::Result<bam::Record> {
            let header = sam::Header::default();

            let record_buf = sam::alignment::RecordBuf::builder()
                .set_data(
                    [(Tag::ALIGNMENT_HIT_COUNT, alignment_hit_count)]
                        .into_iter()
                        .collect(),
                )
                .build();

            let mut writer = bam::io::Writer::from(Vec::new());
            writer.write_alignment_record(&header, &record_buf)?;

            let src = writer.into_inner();
            let mut reader = bam::io::Reader::from(&src[..]);
            let mut record = bam::Record::default();
            reader.read_record(&mut record)?;

            Ok(record)
        }

        let record = build_record(Value::from(-1))?;
        assert!(is_unique_record(&record)?);

        let record = build_record(Value::from(0))?;
        assert!(is_unique_record(&record)?);

        let record = build_record(Value::from(1))?;
        assert!(is_unique_record(&record)?);

        let record = build_record(Value::from(2))?;
        assert!(!is_unique_record(&record)?);

        let record = build_record(Value::from("atlas"))?;
        assert!(matches!(
            is_unique_record(&record),
            Err(e) if e.kind() == io::ErrorKind::InvalidData
        ));

        Ok(())
    }
}
