#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub hostname: String,
    pub client_id: String,
    pub qos: i32,
    pub username: String,
    pub password: String,
    pub subscribed_topics: Vec<Topics>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Topics {
    pub name: String,
    pub qos: i32,
}

impl Config {
    fn new() -> Self {
        Config {
            hostname: "mqtt://localhost:1883".to_string(),
            client_id: "TEMP_CLIENT".to_string(),
            qos: 0,
            username: "TEMP_USER".to_string(),
            password: "temppassword".to_string(),
            subscribed_topics: vec![
                Topics {
                    name: "topic_1".to_string(),
                    qos: 0,
                },
                Topics {
                    name: "topic_2".to_string(),
                    qos: 0,
                },
            ],
        }
    }

    pub fn load(f: String) -> Self {
        let config_file = std::fs::File::open(f).expect("Could not open file.");
        let config: Result<Config, serde_yml::Error> = serde_yml::from_reader(config_file);
        config.unwrap_or_else(|_| Self::new())
    }

    pub fn parse_mqtt_topics(&self) -> (Vec<String>, Vec<i32>) {
        let topics: Vec<String> = self
            .subscribed_topics
            .iter()
            .map(|x| x.name.clone())
            .collect::<Vec<String>>();
        let qos: Vec<i32> = self.subscribed_topics.iter().map(|x| x.qos).collect();
        (topics, qos)
    }
}
