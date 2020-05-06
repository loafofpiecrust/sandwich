mod behavior;
mod grammar;
mod sandwich;
mod state;
mod client;

use anyhow;
use serde_yaml;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{self, prelude::*};
use sandwich::{Ingredient, Sandwich};
use grammar::WordFunction;
use client::Client;

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
        let verb = client.context.dictionary.first_word_in_class(WordFunction::Desire);
        let line = format!("{} {}", word, verb);
        // display the sentence for debugging.
        let sentence = grammar::sentence(line.as_bytes(), &client.context);
        dbg!(sentence);

        let (response, sandwich) = server.respond(&line);
        println!("{}", response);
        dbg!(sandwich);
    }

    // Say goodbye!
    client.end_order(&mut server);

    Ok(())
}
