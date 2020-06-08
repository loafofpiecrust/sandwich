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
use palette::{named, Srgb};
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use serde::{Deserialize, Serialize};

pub trait Operation: std::fmt::Debug {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich;
    fn reverse(&self) -> Box<dyn Operation>;
    fn encode(&self, lang: &Language) -> String;
}

/// Add an ingredient to a sandwich, at the very end or relative to another ingredient.
#[derive(Debug)]
pub struct Add(pub Ingredient, pub Relative);
impl Operation for Add {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
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
        personality.spite += 0.05;
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
                let p = lang.dictionary.word_for_def(WordFunction::Before);
                let n = lang.dictionary.ingredients.to_word(&other, String::new());
                format!("{} {} ", n.unwrap(), p.0)
            }
            Relative::After(other) => {
                let p = lang.dictionary.word_for_def(WordFunction::After);
                let n = lang.dictionary.ingredients.to_word(&other, String::new());
                format!("{} {} ", n.unwrap(), p.0)
            }
            Relative::Top => String::new(),
        };

        // Get the word for our verb and ingredient.
        let verb = lang.dictionary.word_for_def(WordFunction::Desire);
        let obj = lang.dictionary.ingredients.to_word(&self.0, String::new());
        // TODO Change by word order.
        format!("{}{} {}", prep, obj.unwrap(), verb.0)
    }
}

#[derive(Debug, Clone)]
pub enum Relative {
    Before(Ingredient),
    After(Ingredient),
    Top,
}
impl Relative {
    pub fn from_def(def: WordFunction, ingredient: Ingredient) -> Self {
        match def {
            WordFunction::Before => Relative::Before(ingredient),
            WordFunction::After => Relative::After(ingredient),
            _ => Relative::Top,
        }
    }
}

/// Remove the given ingredient from a sandwich.
#[derive(Debug)]
pub struct Remove(Ingredient);
impl Operation for Remove {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        let mut ingredients = sandwich.ingredients;
        if let Some(idx) = ingredients.iter().position(|x| x.name == self.0.name) {
            ingredients.remove(idx);
        }
        // Ingredient removal raises spite!
        personality.spite += 0.1;
        Sandwich {
            ingredients,
            ..sandwich
        }
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Add(self.0.clone(), Relative::Top))
    }
    fn encode(&self, lang: &Language) -> String {
        let neg = lang.dictionary.word_for_def(WordFunction::Negation);
        format!("{} {}", neg.0, self.reverse().encode(lang))
    }
}

#[derive(Debug, Clone)]
pub struct Finish;
impl Operation for Finish {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        Sandwich {
            complete: true,
            ..sandwich
        }
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(self.clone())
    }
    fn encode(&self, lang: &Language) -> String {
        let bye = lang.dictionary.word_for_def(WordFunction::Greeting);
        bye.0.into()
    }
}

/// Applies an operation on a sandwich multiple times.
#[derive(Debug)]
pub struct Repeat(pub u32, pub Box<dyn Operation>);
impl Operation for Repeat {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        let mut sandwich = sandwich;
        for _ in 0..self.0 {
            sandwich = self.1.apply(sandwich, personality);
        }
        sandwich
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Self(self.0, self.1.reverse()))
    }
    fn encode(&self, lang: &Language) -> String {
        let num = lang.dictionary.word_for_num(self.0);
        format!("{} {}", num.0, self.1.encode(lang))
    }
}

/// Applies two operations sequentially on a sandwich.
#[derive(Debug)]
pub struct Compound(pub Box<dyn Operation>, pub Box<dyn Operation>);
impl Operation for Compound {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        // Apply the inner operations sequentially.
        self.1
            .apply(self.0.apply(sandwich, personality), personality)
    }
    // TODO This could also just reverse the order of it?
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Compound(self.0.reverse(), self.1.reverse()))
    }
    fn encode(&self, lang: &Language) -> String {
        let conj = lang.dictionary.word_for_def(WordFunction::And);
        // Conjunction goes between two sub-phrases.
        format!("{} {} {}", self.0.encode(lang), conj.0, self.1.encode(lang))
    }
}

/// Applies to (roughly) the duration of an order, and means this ingredient
/// should never be added to the sandwich. Expressed with allergen terminology.
#[derive(Debug)]
pub struct NeverAdd(pub Ingredient);
impl Operation for NeverAdd {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        todo!()
    }
    fn reverse(&self) -> Box<dyn Operation> {
        todo!()
    }
    fn encode(&self, lang: &Language) -> String {
        // adjective: "I am allgergic to X"
        // or noun: "X is an allergy"
        // or verb: "I react to X"
        // or reverse verb: "X causes reaction"
        todo!()
    }
}

// #[derive(Debug)]
// pub struct ChangeBackground(pub String);
// impl Operation for ChangeBackground {
//     fn apply(&self, sandwich: Sandwich) -> Sandwich {
//         Sandwich {
//             background_color: self.0.clone(),
//             ..sandwich
//         }
//     }
//     fn reverse(&self) -> Box<dyn Operation> {
//         todo!()
//     }
//     fn encode(&self, lang: &Language) -> String {

//         todo!()
//     }
// }

// pub struct Negate(Box<dyn Operation>);
// impl Operation for Negate {
//     fn apply(self, sandwich: Sandwich) -> Sandwich {
//         self.0.reverse().apply(sandwich)
//     }
//     fn reverse(self) -> Box<dyn Operation> {
//         self.0
//     }
// }

pub struct Order {
    forgotten: Vec<Box<dyn Operation>>,
    history: Vec<Box<dyn Operation>>,
    personality: Personality,
    desired: Sandwich,
}
impl Order {
    pub fn new(lang: &Language) -> Self {
        Self {
            forgotten: Vec::new(),
            history: Vec::new(),
            personality: Personality::new(lang),
            // TODO Pick a sandwich based on our personality.
            desired: Sandwich::random(&lang.dictionary.ingredients, 7),
        }
    }

    /// Based on the current conversation state and resulting sandwich, choose
    /// an operation to ask our conversation partner to apply to said sandwich.
    pub fn pick_op(&mut self, result: &Sandwich) -> Option<Box<dyn Operation>> {
        let mut rng = thread_rng();

        // If the result has all the ingredients we want, then we're finished.
        let has_all = self
            .desired
            .ingredients
            .iter()
            .all(|x| result.ingredients.contains(x));
        if has_all {
            return None;
        }

        // The basic behavior: pick the next ingredient on the sandwich.
        // Find the top-most shared ingredient between desired and result.
        let last_shared = self
            .desired
            .ingredients
            .iter()
            .enumerate()
            // Mostly filter out allergens.
            .filter(|(idx, x)| {
                !self
                    .personality
                    .allergies
                    .iter()
                    .any(|a| &a.ingredient == *x && rng.gen_bool(a.severity))
            })
            .rfind(|(idx, x)| result.ingredients.contains(x));
        // We want to add the next one!
        let mut next_idx = last_shared.map(|(i, _)| i + 1).unwrap_or(0);
        println!("next index we want: {}", next_idx);

        // Maybe forget this ingredient and move on to the next one.
        if rng.gen_bool(self.personality.forgetfulness) {
            next_idx += 1;
        }

        // TODO When considering a removal, maybe try to do a swap instead.

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
            .personality
            .allergies
            .iter()
            // TODO Use contains logic here instead of exact match, allowing
            // allergies to whole categories.
            .filter(|a| result.ingredients.iter().any(|x| a.ingredient.includes(x)))
            .next();

        if let Some(allergen) = allergen {
            // If the allergy is severe and we aren't shy about it, ask for that
            // ingredient to be removed.
            if rng.gen_bool(allergen.severity) && !rng.gen_bool(self.personality.shyness) {
                return Some(Box::new(Remove(allergen.ingredient.clone())));
            }
        }

        // TODO Change my mind about what I want based on my favorites.
        if rng.gen_bool(self.personality.spontaneity) {
            // If our previous desires contain too few of our favorites, then
            // add one in.
            let any_favs = self.desired.ingredients.iter().any(|x| {
                // Check if one of our favorites includes this ingredient.
                self.personality
                    .favorites
                    .iter()
                    .any(|fav| fav.ingredient.includes(x) && rng.gen_bool(fav.severity))
            });
            if !any_favs && !self.personality.favorites.is_empty() {
                // Pick a random favorite based on their severity.
                // NOTE Assumes every machine has at least one favorite.
                let weights = self.personality.favorites.iter().map(|x| x.severity);
                let dist = WeightedIndex::new(weights).unwrap();
                let pick = dist.sample(&mut rng);
                return Some(Box::new(Add(
                    self.personality.favorites[pick].ingredient.clone(),
                    Relative::Top,
                )));
            }
        }

        // If there are multiple of the ingredient we want, ask for them all at once.

        let next_ingr = self.desired.ingredients.get(next_idx);
        next_ingr.map(|next_ingr| {
            // Number of the ingredient we want in a row
            // TODO Check the whole list of remaining ingredients if order doesn't
            // matter to this machine.
            let adder = Box::new(Add(next_ingr.clone(), Relative::Top));
            let same_count = self
                .desired
                .ingredients
                .iter()
                .skip(next_idx) // If we want index 1, skip just the zeroth.
                .take_while(|x| x == &next_ingr)
                .count();
            if same_count > 1 {
                Box::new(Repeat(same_count as u32, adder)) as Box<dyn Operation>
            } else {
                // If your memory is good and you aren't shy, ask for two ingredients at once.
                let another_one = self.desired.ingredients.get(next_idx + 1);
                if let Some(another_one) = another_one {
                    if !rng.gen_bool(self.personality.forgetfulness)
                        && !rng.gen_bool(self.personality.shyness)
                    {
                        return Box::new(Compound(
                            adder,
                            Box::new(Add(another_one.clone(), Relative::Top)),
                        )) as Box<dyn Operation>;
                    }
                }

                // Default behavior, just add the next ingredient to the top of the sandwich.
                adder as Box<dyn Operation>
            }
        })
    }
}

/// A single message of text and/or a sandwich.
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
