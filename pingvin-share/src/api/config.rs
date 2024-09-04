use dto::{ConfigEntry, ConfigValue};
use serde_json::Number;

pub struct PublicConfiguration {
    entries: Vec<ConfigEntry>,
}

impl PublicConfiguration {
    pub fn new(entries: Vec<ConfigEntry>) -> Self {
        Self { entries }
    }

    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map(|value| &value.value)
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.get(key)
            .map(|v| match v {
                ConfigValue::String(v) => Some(v.as_str()),
                _ => None,
            })
            .flatten()
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key)
            .map(|v| match v {
                ConfigValue::Boolean(v) => Some(*v),
                _ => None,
            })
            .flatten()
    }

    pub fn get_number(&self, key: &str) -> Option<&Number> {
        self.get(key)
            .map(|v| match v {
                ConfigValue::Number(v) => Some(v),
                _ => None,
            })
            .flatten()
    }
}

mod dto {
    use serde::{
        de::{self, Error},
        Deserialize, Deserializer,
    };
    use serde_json::{Number, Value};

    #[derive(Debug, Deserialize)]
    #[serde(tag = "type", content = "value", rename_all = "lowercase")]
    pub enum ConfigValue {
        String(String),

        #[serde(deserialize_with = "ConfigValue::parse_number")]
        Number(Number),

        #[serde(deserialize_with = "ConfigValue::parse_boolean")]
        Boolean(bool),
    }

    impl ConfigValue {
        fn parse_boolean<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
            Ok(match serde::de::Deserialize::deserialize(deserializer)? {
                Value::Bool(b) => b,
                Value::String(s) => s == "true",
                _ => return Err(de::Error::custom("Wrong type, expected boolean")),
            })
        }

        fn parse_number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Number, D::Error> {
            Ok(match serde::de::Deserialize::deserialize(deserializer)? {
                Value::Number(n) => n,
                Value::String(s) => s.parse::<i64>().map_err(Error::custom)?.into(),
                _ => return Err(de::Error::custom("Wrong type, expected number")),
            })
        }
    }

    #[derive(Debug, Deserialize)]
    pub struct ConfigEntry {
        pub key: String,

        #[serde(flatten)]
        pub value: ConfigValue,
    }

    #[cfg(test)]
    mod test {
        use crate::api::{ConfigEntry, ConfigValue};

        #[test]
        fn test_parse() {
            const PAYLOAD: &str = r#"[{"key":"smtp.enabled","value":"true","type":"boolean"},{"key":"general.appName","value":"Sendy","type":"string"},{"key":"general.appUrl","value":"https://sendy.did.science","type":"string"},{"key":"general.showHomePage","value":"false","type":"boolean"},{"key":"general.sessionDuration","value":"2160","type":"number"},{"key":"share.allowRegistration","value":"false","type":"boolean"},{"key":"share.allowUnauthenticatedShares","value":"false","type":"boolean"},{"key":"share.maxExpiration","value":"0","type":"number"},{"key":"share.maxSize","value":"1000000000","type":"number"},{"key":"share.chunkSize","value":"10000000","type":"number"},{"key":"share.autoOpenShareModal","value":"false","type":"boolean"},{"key":"email.enableShareEmailRecipients","value":"true","type":"boolean"},{"key":"smtp.allowUnauthorizedCertificates","value":"true","type":"boolean"},{"key":"oauth.disablePassword","value":"false","type":"boolean"}]"#;
            assert!(serde_json::from_str(&PAYLOAD).is_ok());
        }
    }
}
