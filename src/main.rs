mod audio;
mod behavior;
mod client;
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
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    // First we need to establish communication with a greeting.
    let mut server = Client::default();
    let mut client = Client::default();

    client.add_behavior(Box::new(behavior::Forgetful::default()));

    random_encounter(client, server)
    // interactive(client, server)
}

fn interactive(client: Client, mut server: Client) -> anyhow::Result<()> {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.is_empty() {
            break;
        }

        // display the sentence for debugging.
        let sentence = grammar::sentence(line.as_bytes(), &client.context);
        dbg!(sentence);

        let (response, sandwich) = server.respond(&line);
        println!("{}", response);
        dbg!(sandwich);
    }
    Ok(())
}

fn random_encounter(mut client: Client, mut server: Client) -> anyhow::Result<()> {
    // Initial greeting phase!
    client.start_order(&mut server);

    dbg!(&client.sandwich);

    // List all the ingredients I want.
    while let Some(word) = client.next_phrase() {
        println!("ingredient: {}", word);
        let verb = client
            .context
            .dictionary
            .first_word_in_class(WordFunction::Desire);
        let line = format!("{} {}", word, verb);
        for w in line.split(" ") {
            audio::play_word(w)?;
            thread::sleep(Duration::from_millis(100));
        }
        // display the sentence for debugging.
        let sentence = grammar::sentence(line.as_bytes(), &client.context);
        dbg!(sentence);

        thread::sleep(Duration::from_millis(700));

        let (response, sandwich) = server.respond(&line);
        println!("{}", response);
        for w in response.split(" ") {
            audio::play_word(w)?;
        }
        dbg!(sandwich);

        thread::sleep(Duration::from_millis(500));
    }

    // Say goodbye!
    client.end_order(&mut server);

    Ok(())
}
