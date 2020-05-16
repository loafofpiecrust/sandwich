use crate::grammar::*;
use crate::sandwich::{Ingredient, Sandwich};

pub trait State {
    fn respond(
        &mut self,
        input: &PhraseNode,
        dict: &Dictionary,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>);
}

/// Initial state when not conversing with any other robot
pub struct Idle;
impl State for Idle {
    fn respond(
        &mut self,
        input: &PhraseNode,
        dict: &Dictionary,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>) {
        // Only respond if being properly greeted.
        if let Some(WordFunction::Greeting) = input.main_verb() {
            (
                dict.first_word_in_class(WordFunction::Greeting).into(),
                None,
                Some(Box::new(SandwichOrder::new())),
            )
        } else {
            (
                dict.first_word_in_class(WordFunction::Negation).into(),
                None,
                None,
            )
        }
    }
}

/// Receiving an order for a sandwich
pub struct SandwichOrder {
    sandwich: Sandwich,
}
impl SandwichOrder {
    fn new() -> Self {
        Self {
            sandwich: Sandwich::default(),
        }
    }
}
impl State for SandwichOrder {
    fn respond(
        &mut self,
        input: &PhraseNode,
        dict: &Dictionary,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>) {
        // TODO Process positional phrases here too somehow.
        if let Some(WordFunction::Desire) = input.main_verb() {
            let word = input.object();
            if let Some(entry) = word.map(|o| o.entry.as_ref()).flatten() {
                if entry.function == WordFunction::Greeting {
                    // Represents showing the sandwich to the client.
                    println!("{:?}", self.sandwich);
                    // End the conversation.
                    return (
                        dict.first_word_in_class(WordFunction::Greeting).into(),
                        Some(self.sandwich.clone()),
                        Some(Box::new(Idle)),
                    );
                } else if entry.function == WordFunction::Ingredient {
                    let ingredient = dict.ingredients.from_word(&word.unwrap().word);
                    // TODO: Add the given ingredient to the sandwich order.
                    self.sandwich.ingredients.push(ingredient.clone());
                    return (
                        dict.first_word_in_class(WordFunction::Affirmation).into(),
                        None,
                        None,
                    );
                }
            }
        }
        (
            dict.first_word_in_class(WordFunction::Negation).into(),
            None,
            None,
        )
    }
}
