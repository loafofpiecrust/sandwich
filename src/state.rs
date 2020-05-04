use crate::grammar::*;
use crate::sandwich::Ingredient;

pub trait State {
    fn respond(
        &mut self,
        input: &AnnotatedPhrase,
        dict: &Dictionary,
    ) -> (String, Option<Box<dyn State>>);
}

/// Initial state when not conversing with any other robot
pub struct Idle;
impl State for Idle {
    fn respond(
        &mut self,
        input: &AnnotatedPhrase,
        dict: &Dictionary,
    ) -> (String, Option<Box<dyn State>>) {
        let word = &input[0];
        if let Some(entry) = &word.entry {
            // Only respond if being properly greeted.
            if entry.function == WordFunction::Greeting {
                return (
                    dict.first_word_in_class(WordFunction::Greeting),
                    Some(Box::new(SandwichOrder::new())),
                );
            }
        }
        ("no".to_string(), None)
    }
}

/// Receiving an order for a sandwich
pub struct SandwichOrder {
    sandwich: Vec<Ingredient>,
}
impl SandwichOrder {
    fn new() -> Self {
        Self {
            sandwich: Vec::new(),
        }
    }
}
impl State for SandwichOrder {
    fn respond(
        &mut self,
        input: &AnnotatedPhrase,
        dict: &Dictionary,
    ) -> (String, Option<Box<dyn State>>) {
        let word = &input[0];
        if let Some(entry) = &word.entry {
            if entry.function == WordFunction::Greeting {
                // Represents showing the sandwich to the client.
                println!("{:?}", self.sandwich);
                // End the conversation.
                return (
                    dict.first_word_in_class(WordFunction::Greeting),
                    Some(Box::new(Idle)),
                );
            } else if entry.function == WordFunction::Ingredient {
                let ingredient = dict.ingredients.from_word(&word.word);
                // TODO: Add the given ingredient to the sandwich order.
                self.sandwich.push(ingredient.clone());
                return (dict.first_word_in_class(WordFunction::Affirmation), None);
            }
        }
        ("no".to_string(), None)
    }
}
