use crate::grammar::{self, Context, WordFunction};
use crate::sandwich::{Ingredient, Sandwich};
use crate::Client;
use itertools::Itertools;
use rand;
use rand::prelude::*;

/// Basically a fixed iterator instance.
/// Having the ingredient index lets us develop spatial relationships to other ingredients.
/// TODO If we want to develop temporal relationships, we could store some history/memory here.
#[derive(Clone, Copy, Debug)]
pub struct PositionedIngredient<'a> {
    pub sandwich: &'a Sandwich,
    pub history: &'a [usize],
    pub index: usize,
}

pub enum OrderStatus {
    Ingredient(usize),
    None,
    Finished,
}

pub trait Behavior {
    fn start(&self);
    fn end(&self);
    fn next_ingredient(&mut self, sandwich: &Sandwich, pick: Option<usize>) -> Option<usize>;
}

#[derive(Clone, Default)]
pub struct Forgetful {
    forgotten: Vec<usize>,
}
impl Behavior for Forgetful {
    fn start(&self) {}
    fn end(&self) {}
    fn next_ingredient(&mut self, sandwich: &Sandwich, pick: Option<usize>) -> Option<usize> {
        let mut rng = thread_rng();
        let curr_idx = pick.unwrap_or(0);
        // TODO Chance to remember a forgotten ingredient.
        if rng.gen_bool(0.5) && !self.forgotten.is_empty() {
            println!("remembering!!");
            Some(self.forgotten.remove(0))
        } else if pick.is_some() && sandwich.ingredients.len() > curr_idx && rng.gen_bool(0.70) {
            println!("forgetting!");
            self.forgotten.push(curr_idx);
            if curr_idx + 1 < sandwich.ingredients.len() {
                Some(curr_idx + 1)
            } else {
                None
            }
        } else {
            pick
        }
    }
}

// pub struct InOrder;
// impl Behavior for InOrder {
//     fn start(&self) {}
//     fn end(&self) {}
//     fn next_ingredient(&mut self, sandwich: &Sandwich, pick: Option<usize>) -> Option<usize> {
//         Some(pick.unwrap_or(0))
//     }
// }

enum Resolvable {
    Resolved(String),
    Unresolved(usize),
}
impl Resolvable {
    fn to_string(self) -> Option<String> {
        if let Resolvable::Resolved(s) = self {
            Some(s)
        } else {
            None
        }
    }
}

/// TODO Give this two traits! One for parsing, one for encoding!!
/// Each encoder may implement new parsing/encoding features for the language.
pub trait Encoder {
    fn encode(&mut self, context: &Context, item: PositionedIngredient) -> String;
}

/// A root encoder.
pub struct DesireEncoder;
impl Encoder for DesireEncoder {
    fn encode(&mut self, context: &Context, item: PositionedIngredient) -> String {
        let ingredient = &item.sandwich.ingredients[item.index];
        let obj = context
            .dictionary
            .ingredients
            .to_word(&ingredient, String::new())
            .unwrap();
        let verb = context.dictionary.first_word_in_class(WordFunction::Desire);
        format!("{} {}", obj, verb)
    }
}

pub struct RelativeEncoder {
    inner: Box<dyn Encoder>,
}
impl RelativeEncoder {
    pub fn new(inner: impl Encoder + 'static) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }
}
impl Encoder for RelativeEncoder {
    fn encode(&mut self, context: &Context, item: PositionedIngredient) -> String {
        let ingredient = &item.sandwich.ingredients[item.index];
        let last_order = *item.history.last().unwrap_or(&0);
        println!("Encoding relative maybe? {:?}", item);
        if item.index != last_order && item.index != last_order + 1 && item.index > 0 {
            println!("Encoding relative!");
            let previous = &item.sandwich.ingredients[item.index - 1];
            let prev_word = context
                .dictionary
                .ingredients
                .to_word(&previous, String::new())
                .unwrap();
            let prep = context.dictionary.first_word_in_class(WordFunction::After);
            // Syntax is: V'[PP[NP P] V'...]
            format!(
                "{} {} {}",
                prev_word,
                prep,
                self.inner.encode(context, item)
            )
        } else {
            self.inner.encode(context, item)
        }
    }
}
