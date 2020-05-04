mod behavior;
mod grammar;
mod message;
mod sandwich;
mod state;

use anyhow;
use serde_yaml;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{self, prelude::*};

fn main() -> anyhow::Result<()> {
    // A language first needs a set of phonemes to build syllables out of.
    // let consonant_phonemes = vec!["p", "t", "k", "h", "m", "n", "w", "l"];
    // let vowel_phonemes = vec!["a", "i", "u", "o", "e"];

    // Syllable structure is CV to start.
    // Word order starts out as strict SOV.
    // Build a table of all possible syllables.
    // let syllables = all_syllables(&consonant_phonemes, &vowel_phonemes);

    // First we need to establish communication with a greeting.
    let mut server = Client::default();
    let mut client = Client::default();

    client.add_behavior(Box::new(behavior::Forgetful));

    // Initial greeting phase!
    client.start_order(&mut server);

    // List all the ingredients I want.
    while let Some(word) = client.next_phrase() {
        println!("ingredient: {}", word);
        let line = format!("{} nu", word);
        let phrase = grammar::phrase(line.as_bytes());
        if let Ok((_, phrase)) = phrase {
            let annotated = grammar::annotate(&phrase, &client.context);
            let sentence = grammar::sentence(line.as_bytes(), &client.context);
            println!("{:?}", sentence);
            // println!("{:?}", phrase);
            // println!("{:?}", annotated);

            let response = server.context.respond(&annotated);
            println!("{}", response);
        }
    }

    // Say goodbye!
    client.end_order(&mut server);

    Ok(())
}

#[derive(Default)]
pub struct Client {
    pub context: grammar::Context,
    sandwich: Option<sandwich::Sandwich>,
    behaviors: Vec<Box<dyn behavior::Behavior>>,
    filled_ingredients: Vec<sandwich::Ingredient>,
}
impl Client {
    pub fn invent_sandwich(&self) -> sandwich::Sandwich {
        sandwich::Sandwich::random(&self.context.dictionary.ingredients, 5)
    }
    pub fn add_behavior(&mut self, b: Box<dyn behavior::Behavior>) {
        self.behaviors.push(b);
    }
    pub fn start_order(&mut self, other: &mut Client) {
        self.sandwich = Some(self.invent_sandwich());
        self.greet(other);
        for b in &self.behaviors {
            b.start();
        }
    }
    pub fn end_order(&mut self, other: &mut Client) {
        for b in &self.behaviors {
            b.end();
        }
        self.greet(other);
        self.sandwich = None;
    }
    fn greet(&self, other: &mut Client) {
        let greeting = grammar::phrase("loha".as_bytes());
        if let Ok((_, phrase)) = greeting {
            let parsed = grammar::annotate(&phrase, &self.context);
            let resp = other.context.respond(&parsed);
            println!("{}", resp);
        }
    }
    pub fn next_phrase(&mut self) -> Option<String> {
        println!("sandwich: {:?}", self.sandwich);
        let sandwich = self.sandwich.as_ref().unwrap();
        let mut ingredients_left: Vec<_> = sandwich
            .ingredients
            .iter()
            // Take only the trailing ingredients that aren't filled yet.
            // This supports forgetting ingredients if skipped.
            .rev()
            .take_while(|x| !self.filled_ingredients.contains(x))
            .collect();
        ingredients_left.reverse();

        println!("ingr left: {:?}", ingredients_left);

        if ingredients_left.is_empty() {
            return None;
        }

        let mut next_ingredient = ingredients_left[0];
        // Allow behavior to change what the next ingredient might be.
        for b in &self.behaviors {
            next_ingredient = b.next_ingredient(&ingredients_left, next_ingredient);
        }
        self.filled_ingredients.push(next_ingredient.clone());
        self.context
            .dictionary
            .ingredients
            .to_word(next_ingredient, "".into())
    }

    /// Returns a score for the match between the sandwich we wanted and the sandwich we got.
    /// TODO A low enough score may warrant revisions, depending on how shy this client is.
    pub fn judge_sandwich(&self, result: &sandwich::Sandwich) -> f64 {
        // For now, just count the number of ingredients that match.
        // TODO Count the number of matching *morphemes*.
        if let Some(sandwich) = &self.sandwich {
            // Number of correct ingredients we did ask for.
            let tp = sandwich
                .ingredients
                .iter()
                .filter(|x| result.ingredients.contains(x))
                .count();
            // Number of extra ingredients we didn't ask for.
            let fp = result
                .ingredients
                .iter()
                .filter(|x| !sandwich.ingredients.contains(x))
                .count();
            (tp as f64) / (fp as f64)
        } else {
            0.0
        }
    }
}
