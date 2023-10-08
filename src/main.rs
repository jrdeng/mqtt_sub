use clap::Parser;
use futures::{executor::block_on, stream::StreamExt};
use paho_mqtt as mqtt;
use std::{process, time::Duration};
use uuid::Uuid;

#[derive(Parser)]
// #[command(disable_help_flag=true)]
struct Cli {
    #[arg(long, value_name = "HOST")]
    host: String,

    #[arg(long, value_name = "PORT", default_value = "1883")]
    port: i32,

    #[arg(long = "topic", value_name = "TOPIC")]
    topics: Vec<String>,
}

fn main() {
    let cli = Cli::parse();

    let host = cli.host;
    let port = cli.port;
    let server_uri = format!("tcp://{}:{}", host, port);
    let client_id = Uuid::new_v4().to_string();

    let topics: Vec<&str> = cli.topics.iter().map(String::as_str).collect();
    if topics.is_empty() {
        println!("Error: topic must be specified! via --topic=abc");
        process::exit(-1);
    }
    let qos = vec![1; topics.len()];

    println!("Creating client to: {:?}, client_id: {:?}", server_uri, client_id);
    let create_opts = mqtt::CreateOptionsBuilder::new()
        .server_uri(&server_uri)
        .client_id(&client_id)
        .finalize();
    let mut cli = mqtt::AsyncClient::new(create_opts).unwrap_or_else(|e| {
        println!("Error on creating client: {:?}", e);
        process::exit(-1);
    });

    if let Err(err) = block_on(async {
        // Get message stream before connecting.
        let mut strm = cli.get_stream(25);

        let conn_opts = mqtt::ConnectOptionsBuilder::with_mqtt_version(mqtt::MQTT_VERSION_5)
            .clean_start(true)
            .finalize();

        // Make the connection to the broker
        cli.connect(conn_opts).await?;

        println!("Subscribing to topics: {:?}", topics);
        cli.subscribe_many(&topics, &qos).await?;

        // Just loop on incoming messages.
        println!("Waiting for messages...");
        while let Some(msg_opt) = strm.next().await {
            if let Some(msg) = msg_opt {
                if msg.retained() {
                    print!("(R) ");
                }
                println!("{}", msg);
            } else {
                // A "None" means we were disconnected. Try to reconnect...
                println!("Lost connection. Attempting reconnect.");
                while let Err(err) = cli.reconnect().await {
                    println!("Error reconnecting: {}", err);
                    // For tokio use: tokio::time::delay_for()
                    async_std::task::sleep(Duration::from_millis(1000)).await;
                }
            }
        }

        // Explicit return type for the async block
        Ok::<(), mqtt::Error>(())
    }) {
        println!("Error: {:?}", err);
    }
}
