use std::error::Error;
use std::str::FromStr;

use serde::de;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub enum ExpireDuration {
    Never,

    Seconds(u64),
    Minutes(u64),
    Hour(u64),
    Days(u64),
    Week(u64),
    Month(u64),
    Year(u64),
}

impl<'de> Deserialize<'de> for ExpireDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match serde::de::Deserialize::deserialize(deserializer)? {
            Value::String(s) => {
                Self::from_str(s.as_str()).map_err(|_| de::Error::custom("invalid value"))?
            }
            _ => return Err(de::Error::custom("Wrong type, expected string")),
        })
    }
}

impl Serialize for ExpireDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl ExpireDuration {
    pub fn to_string(self) -> String {
        let (value, unit) = match self {
            Self::Never => return "never".to_string(),
            Self::Seconds(v) => (v, "second"),
            Self::Minutes(v) => (v, "minute"),
            Self::Hour(v) => (v, "hour"),
            Self::Days(v) => (v, "day"),
            Self::Week(v) => (v, "week"),
            Self::Month(v) => (v, "month"),
            Self::Year(v) => (v, "year"),
        };

        format!("{}-{}", value, unit)
    }
}

impl FromStr for ExpireDuration {
    type Err = Box<dyn Error + Send + Sync + 'static>;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "never" {
            return Ok(Self::Never);
        }

        let (value, unit) = value.split_once("-").ok_or(format!("missing '-'"))?;
        let value = value.parse::<u64>()?;
        Ok(match unit {
            "second" | "seconds" => Self::Seconds(value),
            "minute" | "minutes" => Self::Minutes(value),
            "hour" | "hours" => Self::Hour(value),
            "day" | "days" => Self::Days(value),
            "week" | "weeks" => Self::Week(value),
            "month" | "months" => Self::Month(value),
            "year" | "years" => Self::Year(value),
            unit => return Err(format!("invalid unit '{}'", unit).into()),
        })
    }
}

#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareSecurityOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    max_views: Option<usize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    password: Option<String>,
}
