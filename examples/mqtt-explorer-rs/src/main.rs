#![allow(dead_code)]
use clap::Parser;
use dotenv::dotenv;
use indicatif::HumanBytes;
use paho_mqtt::{self as mqtt};
use serde_json::Value;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::{thread, time::Duration, time::Instant};
mod cli;
mod mqtt_config;
use env_logger::Env;
use log::{error, info};
use rand::Rng;
use tabled::{Table, Tabled, settings::Style};

// Default MQTT topics and QoS
// Use `#` for the string globbing i.e. "weather/#" -> subscribe to all "weather" topics
const DEFAULT_TOPICS: &[&str] = &["topic_1", "topic_2"];
const DEFAULT_QOS: &[i32] = &[0];

#[derive(Debug, Tabled)]
struct Output<'a> {
    #[tabled(rename = "Subscribed Topics")]
    subscribed_topics: i32,
    #[tabled(rename = "Total MQTT Messages")]
    total_messages: i32,
    #[tabled(rename = "Total Tags")]
    total_tags: i32,
    #[tabled(rename = "Capture Duration")]
    capture_duration: i32,
    #[tabled(rename = "MQTT Messages/Seconds")]
    mqtt_mps: i32,
    #[tabled(rename = "MQTT Tags/Seconds")]
    mqtt_tps: i32,
    #[tabled(rename = "MQTT Throughput (Bytes/Seconds)")]
    mqtt_throughput: &'a str,
}

fn main() {
    // Initialize the logger from the environment
    // env_logger::init();
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    // Load environment variable
    dotenv().ok();

    // Generate random number
    let mut rng = rand::rng();

    // Parse CLI
    let cli = cli::Cli::parse();

    let config_file: String = cli.config.unwrap_or_else(|| "config.yaml".to_string());
    let capture_duration = cli.duration.unwrap_or(10);
    let output_file = cli
        .output
        .unwrap_or_else(|| format!("output-{}.json", rng.random::<u32>()).to_string());

    // Load and parse the config file
    let cfg = mqtt_config::Config::load(config_file.clone());
    let hostname = cfg.hostname.clone();
    let client_id = cfg.client_id.clone();
    let username = cfg.username.clone();
    let password = cfg.password.clone();
    let subscribed_topics = cfg.subscribed_topics.clone();

    let (topics, qos) = cfg.parse_mqtt_topics();

    // Create a client creation option object
    // This is used to pass further information during the client creation process
    let client_options = mqtt::CreateOptionsBuilder::new()
        .server_uri(&hostname)
        .client_id(client_id)
        .finalize();

    // Create an SSL options
    // We are not going to use a CA cert to authenticate the identity of the broker
    // hence, we tell paho to not bother trying to authenticate the broker
    let ssl = mqtt::SslOptionsBuilder::new()
        .enable_server_cert_auth(false)
        .ssl_version(paho_mqtt::SslVersion::Default)
        .finalize();

    // Create the MQTT client
    let client = mqtt::Client::new(client_options).expect("Error during client creation");

    // Create a connection option object to configure the username and other information
    let connection_options = mqtt::ConnectOptionsBuilder::new()
        .clean_session(true)
        .user_name(username)
        .password(password)
        .ssl_options(ssl)
        .finalize();

    // Connect to the MQTT broker
    client
        .connect(connection_options)
        .expect("Failed to connect to broker");

    info!("Connected to the broker {}!", &hostname);

    // Subscribe to the topic multiple topics - same qos for every topic
    client
        .subscribe_many(&topics, &qos)
        .expect("Failed to subscribe");

    for subs in subscribed_topics.iter() {
        info!("Subscribed to topic: {} with QoS: {}", subs.name, subs.qos);
    }

    // Starts the client receiving messages
    let rx_queue = client.start_consuming();

    // Create a container to get the result
    let mut res: Vec<serde_json::Value> = vec![];
    let mut size: usize = 0;
    let mut total_tags: usize = 0;

    // Create a thread that stays pending over incoming messages.
    let handle = thread::spawn(move || {
        let start = Instant::now();
        info!("Capturing MQTT messages for {} seconds!", capture_duration);
        let mut rconn_attempt: usize = 0;
        for mqttmsg in rx_queue.iter() {
            if let Some(mqttmsg) = mqttmsg {
                // get payload size
                let s = size_of_val(mqttmsg.payload());
                size += s;

                // serialize byte stream to JSON value and push to result vectore
                let v: Value = serde_json::from_slice(mqttmsg.payload()).unwrap();
                res.push(v.clone());

                // convert value into iterables
                v.as_array().unwrap().iter().for_each(|x| {
                    // get the length of the tags
                    let taglen = x.as_object().unwrap().len();
                    total_tags += taglen;
                });
            } else {
                // If receive "None", wait for message...
                // If error, attempt to reconnect
                while let Err(err) = client.reconnect() {
                    rconn_attempt += 1;
                    error!("Error reconnecting #{}: {}", rconn_attempt, err);
                    thread::sleep(std::time::Duration::from_secs(1));
                }
            }
            let elapsed = start.elapsed();
            if elapsed > std::time::Duration::from_secs(capture_duration as u64) {
                break;
            }
            // Print the elapsed time every second
            info!(
                "Capturing {} MQTT Events | Total Message Size: {} bytes | Elapsed time in seconds: {}",
                res.len(),
                HumanBytes(size as u64),
                elapsed.as_secs()
            );
        }
        let duration = start.elapsed();
        // returning back from thread
        (res, size, duration, total_tags)
    });

    // Keep the program alive for a few seconds to receive messages
    thread::sleep(Duration::from_secs(15));

    // try getting the results by joining handle
    let (res, size, duration, total_tags) = handle.join().expect("Failed to join thread");

    // Write to file
    let file = File::create(&output_file).unwrap();
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &res).unwrap();
    writer.flush().unwrap();
    info!("Captured messages written to {}", output_file);

    // "Disconnect" from the broker
    info!("Disconnected from the broker!");

    // Print Statistics to STDOUT
    let output = Output {
        subscribed_topics: topics.len() as i32,
        total_messages: res.len() as i32,
        total_tags: total_tags as i32,
        capture_duration: duration.as_secs() as i32,
        mqtt_mps: (res.len() as u64 / duration.as_secs()) as i32,
        mqtt_tps: (total_tags as u64 / duration.as_secs()) as i32,
        mqtt_throughput: &HumanBytes(size as u64 / duration.as_secs()).to_string(),
    };
    let mut table = Table::kv(vec![output]);
    table.with(Style::modern().remove_horizontal());

    println!("{}", table);
}
