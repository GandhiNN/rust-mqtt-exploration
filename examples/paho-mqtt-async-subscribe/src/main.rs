use futures::{executor::block_on, stream::StreamExt};
use paho_mqtt as mqtt;
use serde_json::Value;
use std::{env, process, time::Duration};

const TOPICS: &[&str] = &["Topic1/#", "Topic2/weather"];

const QOS: &[i32] = &[0, 0];

fn main() {
    // initialize the logger from the environment
    env_logger::init();

    // Set vars
    let hostname = "mqtt://localhost:1883";
    let client_id = "TEMP_CLIENT_ID";
    let username = "DEFAULT_USER";
    let password = "DEFAULT_PASSWORD";

    // use `ssl` instead of `mqtt`
    let host = env::args().nth(1).unwrap_or_else(|| hostname.to_string());

    println!("Connecting to the MQTT server at '{}'", host);

    // Create the client. Use a Client ID for a persistent session.
    // A real system should try harder to use a unique ID.
    let create_opts = mqtt::CreateOptionsBuilder::new_v3()
        .server_uri(host)
        .client_id(client_id)
        .finalize();

    // Create an SSL options
    // We are not going to use a CA cert to authenticate the identity of the broker
    // so tell paho to not bother trying to authenticate the broker
    let ssl = mqtt::SslOptionsBuilder::new()
        .enable_server_cert_auth(false)
        .finalize();

    // Create the client connection
    let mut client = mqtt::AsyncClient::new(create_opts).unwrap_or_else(|e| {
        println!("Error creating the client: {:?}", e);
        process::exit(1);
    });

    if let Err(err) = block_on(async {
        // Get message stream before connecting
        let mut strm = client.get_stream(5000);

        // Create the connection options for the connection
        // explicitly requesting MQTT v3.x
        let conn_opts = mqtt::ConnectOptionsBuilder::new_v3()
            .keep_alive_interval(Duration::from_secs(30))
            .clean_session(false)
            .user_name(username)
            .password(password)
            .ssl_options(ssl)
            .finalize();

        // Make the connection to the broker
        client.connect(conn_opts).await?;

        println!("Subscribing to topics: {:?}", TOPICS);
        client.subscribe_many(TOPICS, QOS).await?;

        // Just loop on incoming messages
        println!("Waiting for messages...");

        let mut rconn_attempt: usize = 0;

        // Create a container to get the result
        let mut res: Vec<Value> = vec![];

        // Note that we are not providing a way to cleanly shut down
        // and disconnect. Therefore, when we kill this app (with a ^C or whatever)
        // the server will get an unexpected drop
        while let Some(msg_opt) = strm.next().await {
            if let Some(msg) = msg_opt {
                let v: Value = serde_json::from_slice(msg.payload()).unwrap();
                res.push(v);
            } else {
                // If receive "None", wait for message...
                // If error, attempt to reconnect
                while let Err(err) = client.reconnect().await {
                    rconn_attempt += 1;
                    println!("Error reconnecting #{}: {}", rconn_attempt, err);
                    // for tokio use: tokio::time::delay_for()
                    async_std::task::sleep(Duration::from_secs(1)).await;
                }
                println!("Reconnected");
            }
        }
        // Explicit return type for the async block
        Ok::<(), mqtt::Error>(())
    }) {
        eprintln!("{}", err);
    }
}
