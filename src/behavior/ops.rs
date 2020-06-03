//! A machine ordering a sandwich first comes up with what they want.
//! Let's assume for now that there are no presets, they come up with their own recipe.
//!
//! Sandwich struct represents the current state of the conversation, which may soon
//! contain more than simply a list of ingredients.
//!
//! A: I want a sandwich => B: okay
//! A: Add(WheatBread, Relative::End)
//!    => encodes as "I want wheat bread"
//!    => B receives "I want wheat bread"
//!    => decodes it: op = Add(WheatBread, Relative::End)
//!    => sandwich = op.apply(sandwich)
//!
//! The negation particle reverses the operation if found.
//! Once we implement nested operators, position and order of negation may matter.
//! "no avocado want" => Add(Avocado, Relative::End).reverse() => Remove(Avocado)
//!
//! Basically, a VerbPhrase maps to an Operation that we can apply.
//! Modifiers atop that verb phrase may either nest the operation, reverse it, or modify
//! it in other arbitrary ways.

use crate::{
    behavior::Personality,
    client::Language,
    grammar::WordFunction,
    sandwich::{Ingredient, Sandwich},
};
use async_std::net::TcpStream;
use async_std::prelude::*;
use bincode::deserialize;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

struct Weights {
    adpositions: f64,
}

pub trait Operation: std::fmt::Debug {
    fn apply(&self, sandwich: Sandwich) -> Sandwich;
    fn reverse(&self) -> Box<dyn Operation>;
    fn encode(&self, lang: &Language) -> String;
}

/// Add an ingredient to a sandwich, at the very end or relative to another ingredient.
#[derive(Debug)]
pub struct Add(pub Ingredient, pub Relative);
impl Operation for Add {
    fn apply(&self, sandwich: Sandwich) -> Sandwich {
        let mut ingr = sandwich.ingredients;
        let idx = match &self.1 {
            Relative::Before(other) => ingr.iter().position(|x| x.name == other.name),
            Relative::After(other) => ingr
                .iter()
                .position(|x| x.name == other.name)
                .map(|x| x + 1),
            Relative::Top => Some(ingr.len()),
        };
        if let Some(idx) = idx {
            ingr.insert(idx, self.0.clone());
        }
        Sandwich {
            ingredients: ingr,
            ..sandwich
        }
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Remove(self.0.clone()))
    }
    fn encode(&self, lang: &Language) -> String {
        // Encode prepositional phrase.
        // TODO Use language weight for whether to actually use the adposition.
        let prep = match &self.1 {
            Relative::Before(other) => {
                let p = lang.dictionary.first_word_in_class(WordFunction::Before);
                let n = lang.dictionary.ingredients.to_word(&other, String::new());
                format!("{} {} ", n.unwrap(), p.0)
            }
            Relative::After(other) => {
                let p = lang.dictionary.first_word_in_class(WordFunction::After);
                let n = lang.dictionary.ingredients.to_word(&other, String::new());
                format!("{} {} ", n.unwrap(), p.0)
            }
            Relative::Top => String::new(),
        };

        // Get the word for our verb and ingredient.
        let verb = lang.dictionary.first_word_in_class(WordFunction::Desire);
        let obj = lang.dictionary.ingredients.to_word(&self.0, String::new());
        // TODO Change by word order.
        format!("{}{} {}", prep, obj.unwrap(), verb.0)
    }
}

#[derive(Debug)]
pub enum Relative {
    Before(Ingredient),
    After(Ingredient),
    Top,
}

/// Remove the given ingredient from a sandwich.
#[derive(Debug)]
pub struct Remove(Ingredient);
impl Operation for Remove {
    fn apply(&self, sandwich: Sandwich) -> Sandwich {
        let mut ingredients = sandwich.ingredients;
        if let Some(idx) = ingredients.iter().position(|x| x.name == self.0.name) {
            ingredients.remove(idx);
        }
        Sandwich {
            ingredients,
            ..sandwich
        }
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Add(self.0.clone(), Relative::Top))
    }
    fn encode(&self, lang: &Language) -> String {
        let neg = lang.dictionary.first_word_in_class(WordFunction::Negation);
        format!("{} {}", neg.0, self.reverse().encode(lang))
    }
}

#[derive(Debug, Clone)]
pub struct Finish;
impl Operation for Finish {
    fn apply(&self, sandwich: Sandwich) -> Sandwich {
        sandwich
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(self.clone())
    }
    fn encode(&self, lang: &Language) -> String {
        let bye = lang.dictionary.first_word_in_class(WordFunction::Greeting);
        bye.0.into()
    }
}

// pub struct Negate(Box<dyn Operation>);
// impl Operation for Negate {
//     fn apply(self, sandwich: Sandwich) -> Sandwich {
//         self.0.reverse().apply(sandwich)
//     }
//     fn reverse(self) -> Box<dyn Operation> {
//         self.0
//     }
// }

struct Allergy {
    ingredient: Ingredient,
    severity: f64,
}

pub struct Order {
    forgotten: Vec<Box<dyn Operation>>,
    history: Vec<Box<dyn Operation>>,
    allergies: Vec<Allergy>,
    personality: Personality,
    desired: Sandwich,
}
impl Order {
    pub fn new(lang: &Language) -> Self {
        Self {
            forgotten: Vec::new(),
            history: Vec::new(),
            personality: Personality::new(),
            // TODO Pick a sandwich based on our personality.
            desired: Sandwich::random(&lang.dictionary.ingredients, 5),
            allergies: vec![Allergy {
                severity: 0.5,
                ingredient: lang.dictionary.ingredients.random().clone(),
            }],
        }
    }
    pub fn pick_op(&mut self, result: &Sandwich) -> Option<Box<dyn Operation>> {
        let mut rng = thread_rng();
        // The basic behavior: pick the next ingredient on the sandwich.
        // Find the top-most shared ingredient between desired and result.
        let last_shared = self
            .desired
            .ingredients
            .iter()
            .rposition(|x| result.ingredients.contains(x));
        // We want to add the next one!
        let next_idx = last_shared.map(|i| i + 1).unwrap_or(0);
        println!("next index we want: {}", next_idx);

        // There's a mistake if any preceding ingredients aren't in the result sandwich.
        // NOTE disregarding order for the moment.
        let mistake = self
            .desired
            .ingredients
            .iter()
            .take(next_idx)
            .position(|x| !result.ingredients.contains(x));
        // If we aren't shy, try to correct a mistake!
        if mistake.is_some() && !rng.gen_bool(self.personality.shyness) {
            let idx = mistake.unwrap();
            // Pick a preposition to position the missing ingredient where we'd like it.
            // TODO If this machine doesn't care about ordering, then just ask to add it to the end.
            // TODO if idx is zero or if this personality has a preference for Before.
            let rel = if idx == 0 {
                // Find first ingredient that comes after the missing one in our desires.
                let after = self
                    .desired
                    .ingredients
                    .iter()
                    .filter(|x| result.ingredients.contains(x))
                    .next();
                after.map(|b| Relative::Before(b.clone()))
            } else {
                // Find last ingredient in the result that comes before the missing one in our desires.
                let before = self
                    .desired
                    .ingredients
                    .iter()
                    .take(idx)
                    .filter(|x| result.ingredients.contains(x))
                    .last();
                // If we found one, place the missing ingredient after it.
                before.map(|b| Relative::After(b.clone()))
            }
            .unwrap_or(Relative::Top);
            return Some(Box::new(Add(self.desired.ingredients[idx].clone(), rel)));
        }

        // Check for allergens in the result sandwich.
        let allergen = self
            .allergies
            .iter()
            // TODO Use contains logic here instead of exact match, allowing
            // allergies to whole categories.
            .filter(|a| result.ingredients.iter().any(|x| &a.ingredient == x))
            .next();

        if let Some(allergen) = allergen {
            // If the allergy is severe and we aren't shy about it, ask for that
            // ingredient to be removed.
            if rng.gen_bool(allergen.severity) && !rng.gen_bool(self.personality.shyness) {
                return Some(Box::new(Remove(allergen.ingredient.clone())));
            }
        }

        // If the result has all the ingredients we want, then we're finished.
        let has_all = self
            .desired
            .ingredients
            .iter()
            .all(|x| result.ingredients.contains(x));
        if has_all {
            return None;
        }

        // Default behavior, just add the next ingredient to the top of the sandwich.
        Some(Box::new(Add(
            self.desired.ingredients[next_idx].clone(),
            Relative::Top,
        )))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub text: Option<String>,
    pub sandwich: Option<Sandwich>,
}
impl Message {
    pub fn new(text: Option<String>, sandwich: Option<Sandwich>) -> Self {
        Self { text, sandwich }
    }
    /// Max size in bytes of a message.
    const MAX_SIZE: usize = 1024;
    pub async fn recv(stream: &mut TcpStream) -> anyhow::Result<Self> {
        let mut buf = [0u8; Self::MAX_SIZE];
        stream.read(&mut buf).await?;
        Ok(bincode::deserialize_from(&buf as &[u8])?)
    }
    pub async fn send(&self, stream: &mut TcpStream) -> anyhow::Result<()> {
        let mut buf = [0u8; Self::MAX_SIZE];
        bincode::serialize_into(&mut buf as &mut [u8], self)?;
        stream.write(&buf).await?;
        Ok(())
    }
}
