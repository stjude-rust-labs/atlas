use std::fmt;

use serde::{de, Deserialize, Serialize};
use time::{Format, OffsetDateTime};

#[derive(sqlx::Type)]
pub struct Timestampz(pub OffsetDateTime);

impl Serialize for Timestampz {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = self.0.format(Format::Rfc3339);
        serializer.collect_str(&s)
    }
}

impl<'de> Deserialize<'de> for Timestampz {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DateTimeVisitor;

        impl de::Visitor<'_> for DateTimeVisitor {
            type Value = Timestampz;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a date string")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                OffsetDateTime::parse(s, Format::Rfc3339)
                    .map(Timestampz)
                    .map_err(E::custom)
            }
        }

        deserializer.deserialize_str(DateTimeVisitor)
    }
}
