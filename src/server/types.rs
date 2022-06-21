use std::fmt;

use serde::{de, ser, Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(sqlx::Type)]
pub struct Timestampz(pub OffsetDateTime);

impl Serialize for Timestampz {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = self.0.format(&Rfc3339).map_err(ser::Error::custom)?;
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
                OffsetDateTime::parse(s, &Rfc3339)
                    .map(Timestampz)
                    .map_err(E::custom)
            }
        }

        deserializer.deserialize_str(DateTimeVisitor)
    }
}
