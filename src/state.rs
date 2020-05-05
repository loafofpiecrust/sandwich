use crate::grammar::*;
use crate::sandwich::{Sandwich, Ingredient};

pub trait State {
    fn respond(
        &mut self,
        input: &AnnotatedPhrase,
        dict: &Dictionary,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>);
}

/// Initial state when not conversing with any other robot
pub struct Idle;
impl State for Idle {
    fn respond(
        &mut self,
        input: &AnnotatedPhrase,
        dict: &Dictionary,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>) {
        let word = &input[0];
        if let Some(entry) = &word.entry {
            // Only respond if being properly greeted.
            if entry.function == WordFunction::Greeting {
                return (
                    dict.first_word_in_class(WordFunction::Greeting),
                    None,
                    Some(Box::new(SandwichOrder::new())),
                );
            }
        }
        (dict.first_word_in_class(WordFunction::Negation), None, None)
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
        input: &AnnotatedPhrase,
        dict: &Dictionary,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>) {
        let word = &input[0];
        if let Some(entry) = &word.entry {
            if entry.function == WordFunction::Greeting {
                // Represents showing the sandwich to the client.
                println!("{:?}", self.sandwich);
                // End the conversation.
                return (
                    dict.first_word_in_class(WordFunction::Greeting),
                    Some(self.sandwich.clone()),
                    Some(Box::new(Idle)),
                );
            } else if entry.function == WordFunction::Ingredient {
                let ingredient = dict.ingredients.from_word(&word.word);
                // TODO: Add the given ingredient to the sandwich order.
                self.sandwich.ingredients.push(ingredient.clone());
                return (dict.first_word_in_class(WordFunction::Affirmation), None, None);
            }
        }
        (dict.first_word_in_class(WordFunction::Negation), None, None)
    }
}
