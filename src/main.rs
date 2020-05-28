mod audio;
mod behavior;
mod client;
mod comm;
mod display;
mod grammar;
mod sandwich;
mod state;

use anyhow;
use async_std::net::TcpStream;
use async_std::prelude::*;
use behavior::{DesireEncoder, RelativeEncoder};
use client::Client;
use display::Render;
use futures::future::FutureExt;
use futures::pin_mut;
use futures::select;
use rand::prelude::*;
use sandwich::Sandwich;
use std::thread;
use std::time::Duration;

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let c = Client::new();
    let client = comm::find_peer().fuse();
    let server = comm::wait_for_peer().fuse();
    pin_mut!(client, server);
    select! {
        s = client => self::client(s?, c).await,
        s = server => self::server(s?, c).await,
    }
}

async fn client(server: TcpStream, mut client: Client) -> anyhow::Result<()> {
    client.add_behavior(behavior::Forgetful::new(0.3));

    // First we need to establish communication with a greeting.
    random_encounter(client, server).await
}

async fn server(mut stream: TcpStream, mut client: Client) -> anyhow::Result<()> {
    let mut encoder = RelativeEncoder::new(0.8, DesireEncoder);
    loop {
        // Wait for a request,
        let mut buf = [0; 512];
        stream.read(&mut buf).await?;
        let request: String = dbg!(bincode::deserialize(&buf)?);

        // Then respond with words and maybe a sandwich.
        let (resp, sandwich) = client.respond(&request, &mut encoder);
        wait_randomly(300);
        println!("Responding with {}", resp);
        audio::play_phrase(&resp)?;

        buf = [0; 512];
        bincode::serialize_into(&mut buf as &mut [u8], &resp)?;
        stream.write(&buf).await?;

        buf = [0; 512];
        bincode::serialize_into(&mut buf as &mut [u8], &sandwich)?;
        stream.write(&buf).await?;
    }
    thread::sleep(Duration::from_millis(1000));
}

async fn random_encounter(mut client: Client, mut server: TcpStream) -> anyhow::Result<()> {
    let mut encoder = RelativeEncoder::new(0.8, DesireEncoder);

    // Initial greeting phase!
    client.start_order(&mut server).await?;

    dbg!(&client.sandwich);

    // List all the ingredients I want.
    while let Some(line) = client.next_phrase(&mut encoder) {
        client.lang.display.send(Render {
            ingredients: Vec::new(),
            subtitles: client.parse(&line, &mut encoder).unwrap().subtitles(),
        })?;

        // play the word out loud.
        audio::play_phrase(&line)?;

        // Send the other our words.
        let mut buf = [0; 512];
        bincode::serialize_into(&mut buf as &mut [u8], &line)?;
        server.write(&buf).await?;

        // Wait for a response.
        let response: String = {
            let mut buffer = [0; 512];
            server.read(&mut buffer).await?;
            bincode::deserialize(&buffer)?
        };
        let sandwich: Option<Sandwich> = {
            let mut buffer = [0; 512];
            server.read(&mut buffer).await?;
            bincode::deserialize(&buffer)?
        };

        println!("{}", response);
        dbg!(sandwich);

        wait_randomly(800);
    }

    // Say goodbye!
    client.end_order(&mut server).await?;

    thread::sleep(Duration::from_millis(1000));

    Ok(())
}

pub fn wait_randomly(millis: u64) {
    let (min, max) = (millis / 2, millis * 2);
    thread::sleep(Duration::from_millis(thread_rng().gen_range(min, max)));
}
