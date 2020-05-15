mod audio;
mod behavior;
mod client;
mod comm;
mod grammar;
mod sandwich;
mod state;

use anyhow;
use client::Client;
use grammar::WordFunction;
use sandwich::{Ingredient, Sandwich};
use serde_yaml;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{self, prelude::*};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    // server()
    client()
}

fn client() -> anyhow::Result<()> {
    println!("Connecting to peer machine!");
    let server = comm::find_peer()?;
    println!("{:?}", server);
    // First we need to establish communication with a greeting.
    let mut client = Client::new();

    client.add_behavior(Box::new(behavior::Forgetful::default()));

    random_encounter(client, server)
    // interactive(client, server)
}

fn server() -> anyhow::Result<()> {
    let mut client = Client::new();
    let mut stream = comm::wait_for_peer()?;
    loop {
        // Wait for a request,
        let mut buf = [0; 512];
        stream.read(&mut buf)?;
        let request: String = dbg!(bincode::deserialize(&buf)?);

        // Then respond with words and maybe a sandwich.
        let (resp, sandwich) = client.respond(&request);
        println!("Responding with {}", resp);
        audio::play_phrase(&resp)?;

        buf = [0; 512];
        bincode::serialize_into(&mut buf as &mut [u8], &resp)?;
        stream.write(&buf)?;

        buf = [0; 512];
        bincode::serialize_into(&mut buf as &mut [u8], &sandwich)?;
        stream.write(&buf)?;
    }
}

fn interactive(client: Client, mut server: TcpStream) -> anyhow::Result<()> {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.is_empty() {
            break;
        }

        // display the sentence for debugging.
        let sentence = grammar::sentence(line.as_bytes(), &client.context);
        dbg!(sentence);

        // Send the other machine our request.
        server.write(line.as_bytes())?;

        // Then wait for a response...
        // The server will first send their words, then the sandwich (if any).
        // Each piece of this response is 512 bytes, totalling 1KB.
        let response: String = {
            let mut buffer = [0; 512];
            server.read(&mut buffer)?;
            bincode::deserialize(&buffer)?
        };
        let sandwich: Option<Sandwich> = {
            let mut buffer = [0; 512];
            server.read(&mut buffer)?;
            bincode::deserialize(&buffer)?
        };

        println!("{}", response);
        dbg!(sandwich);
    }
    Ok(())
}

fn random_encounter(mut client: Client, mut server: TcpStream) -> anyhow::Result<()> {
    // Initial greeting phase!
    client.start_order(&mut server);

    dbg!(&client.sandwich);

    // List all the ingredients I want.
    while let Some(line) = client.next_phrase() {
        // play the word out loud.
        audio::play_phrase(&line)?;
        // display the sentence for debugging.
        let sentence = grammar::sentence(line.as_bytes(), &client.context);
        dbg!(sentence);

        // Send the other our words.
        let mut buf = [0; 512];
        bincode::serialize_into(&mut buf as &mut [u8], &line)?;
        server.write(&buf)?;

        // Wait for a response.
        let response: String = {
            let mut buffer = [0; 512];
            server.read(&mut buffer)?;
            bincode::deserialize(&buffer)?
        };
        let sandwich: Option<Sandwich> = {
            let mut buffer = [0; 512];
            server.read(&mut buffer)?;
            bincode::deserialize(&buffer)?
        };

        println!("{}", response);
        dbg!(sandwich);

        thread::sleep(Duration::from_millis(500));
    }

    // Say goodbye!
    client.end_order(&mut server);

    Ok(())
}
