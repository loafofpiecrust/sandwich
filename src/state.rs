use crate::grammar::*;
use crate::sandwich::{Ingredient, Sandwich};
use std::sync::mpsc::Sender;

pub trait State {
    fn respond(
        &mut self,
        input: &PhraseNode,
        dict: &Dictionary,
        display: &Sender<Vec<Ingredient>>,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>);
}

/// Initial state when not conversing with any other robot
pub struct Idle;
impl State for Idle {
    fn respond(
        &mut self,
        input: &PhraseNode,
        dict: &Dictionary,
        display: &Sender<Vec<Ingredient>>,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>) {
        // Only respond if being properly greeted.
        if let Some(WordFunction::Greeting) = input.main_verb().and_then(|v| v.definition()) {
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
        display: &Sender<Vec<Ingredient>>,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>) {
        // TODO Process positional phrases here too somehow.
        match input.main_verb().and_then(|v| v.definition()) {
            Some(WordFunction::Desire) => {
                let word = input.object();
                if let Some(WordFunction::Ingredient) =
                    word.and_then(|o| o.entry.as_ref()).map(|e| e.function)
                {
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
            Some(WordFunction::Greeting) => {
                // Represents showing the sandwich to the client.
                println!("{:?}", self.sandwich);
                display.send(self.sandwich.ingredients.clone());
                // End the conversation.
                return (
                    dict.first_word_in_class(WordFunction::Greeting).into(),
                    Some(self.sandwich.clone()),
                    Some(Box::new(Idle)),
                );
            }
            _ => (),
        }
        (
            dict.first_word_in_class(WordFunction::Negation).into(),
            None,
            None,
        )
    }
}
