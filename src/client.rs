use crate::behavior::{
    Behavior, DesireEncoder, Encoder, OrderStatus, PositionedIngredient, RelativeEncoder,
};
use crate::grammar::WordFunction;
use crate::sandwich::{Ingredient, Sandwich};
use crate::{audio, behavior, grammar, sandwich};
use bincode;
use itertools::zip;
use seqalign::{measures::LevenshteinDamerau, Align};
use std::io::{Read, Write};
use std::net::TcpStream;

pub struct Client {
    pub context: grammar::Context,
    behaviors: Vec<Box<dyn Behavior>>,
    encoder: Box<dyn Encoder>,
    pub sandwich: Option<Sandwich>,
    pub history: Vec<usize>,
    next_index: usize,
}
impl Client {
    pub fn new() -> Self {
        Self {
            context: Default::default(),
            behaviors: Vec::new(),
            history: Vec::new(),
            encoder: Box::new(RelativeEncoder::new(DesireEncoder)),
            sandwich: None,
            next_index: 0,
        }
    }
    pub fn invent_sandwich(&self) -> Sandwich {
        Sandwich::random(&self.context.dictionary.ingredients, 5)
    }
    pub fn add_behavior(&mut self, b: Box<dyn Behavior>) {
        self.behaviors.push(b);
    }
    pub fn start_order(&mut self, other: &mut TcpStream) -> anyhow::Result<()> {
        let sammich = self.invent_sandwich();
        self.next_index = 0;
        self.sandwich = Some(sammich);
        self.greet(other)?;
        for b in &self.behaviors {
            b.start();
        }
        Ok(())
    }
    pub fn end_order(&mut self, other: &mut TcpStream) -> anyhow::Result<f64> {
        for b in &self.behaviors {
            b.end();
        }
        let score = self
            .greet(other)?
            .map(|x| self.judge_sandwich(&x))
            .unwrap_or(0.0);
        println!("sandwich score: {}", score);
        self.sandwich = None;
        Ok(score)
    }
    fn greet(&self, other: &mut TcpStream) -> anyhow::Result<Option<Sandwich>> {
        let hello = self
            .context
            .dictionary
            .first_word_in_class(WordFunction::Greeting);
        // Send the phrase over...
        let mut buf = [0; 512];
        bincode::serialize_into(&mut buf as &mut [u8], &hello)?;
        other.write(&buf)?;

        // And wait for a response!
        let resp: String = {
            buf = [0; 512];
            other.read(&mut buf)?;
            bincode::deserialize(&buf).unwrap()
        };
        let sandwich: Option<Sandwich> = {
            buf = [0; 512];
            other.read(&mut buf)?;
            bincode::deserialize(&buf).unwrap()
        };

        println!("{}", resp);
        audio::play_phrase(&resp)?;
        Ok(sandwich)
    }
    pub fn next_phrase(&mut self) -> Option<String> {
        let sandwich = self.sandwich.as_ref().unwrap();

        let mut next_ingredient = Some(self.next_index);
        // Allow behavior to change what the next ingredient might be.
        for b in &mut self.behaviors {
            next_ingredient = b.next_ingredient(sandwich, next_ingredient);
        }

        if let Some(idx) = next_ingredient {
            if idx >= sandwich.ingredients.len() {
                return None;
            }
            let result = Some(self.encoder.encode(
                &self.context,
                PositionedIngredient {
                    sandwich,
                    index: idx,
                    history: &self.history[..],
                },
            ));
            self.history.push(idx);
            self.next_index = self.history.iter().max().unwrap_or(&0) + 1;
            result
        } else {
            None
        }
    }

    /// Returns a score for the match between the sandwich we wanted and the sandwich we got.
    /// TODO A low enough score may warrant revisions, depending on how shy this client is.
    pub fn judge_sandwich(&self, result: &Sandwich) -> f64 {
        // For now, just count the number of ingredients that match.
        // TODO Count the number of matching *morphemes*.
        if let Some(sandwich) = &self.sandwich {
            // Number of correct ingredients we did ask for.
            let measure = LevenshteinDamerau::new(1, 1, 1, 1);
            let alignment = measure.align(&result.ingredients, &sandwich.ingredients);
            1.0 / (alignment.distance() + 1) as f64
        } else {
            0.0
        }
    }

    pub fn respond(&mut self, prompt: &str) -> (String, Option<Sandwich>) {
        if let Ok((_, phrase)) = grammar::phrase(prompt.as_bytes()) {
            let annotated = grammar::annotate(&phrase, &self.context);
            self.context.respond(&annotated)
        } else {
            // TODO use dictionary for all responses.
            (
                self.context
                    .dictionary
                    .first_word_in_class(WordFunction::Negation)
                    .into(),
                None,
            )
        }
    }
}
