use crate::display::{Render, RenderSender};
use crate::grammar::*;
use crate::sandwich::{Ingredient, Sandwich};
use std::sync::mpsc::Sender;

pub trait State {
    // TODO Make this `respond(self) -> Box<dyn State>` so we can move data at the end of
    // a state.
    fn respond(
        &mut self,
        input: &PhraseNode,
        dict: &Dictionary,
        display: &RenderSender,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>);
}

/// Initial state when not conversing with any other robot
pub struct Idle;
impl State for Idle {
    fn respond(
        &mut self,
        input: &PhraseNode,
        dict: &Dictionary,
        display: &RenderSender,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>) {
        // Only respond if being properly greeted.
        if let Some(WordFunction::Greeting) = input.main_verb().and_then(|v| v.definition()) {
            (
                dict.first_word_in_class(WordFunction::Greeting).0.into(),
                None,
                Some(Box::new(SandwichOrder::new())),
            )
        } else {
            (
                dict.first_word_in_class(WordFunction::Negation).0.into(),
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
        display: &RenderSender,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>) {
        // TODO Process positional phrases here too somehow.
        match input.main_verb().and_then(|v| v.definition()) {
            Some(WordFunction::Desire) => {
                let word = input.object();
                if let Some(WordFunction::Ingredient) =
                    word.and_then(|o| o.entry.as_ref()).map(|e| e.function)
                {
                    let ingredient = dict.ingredients.from_word(&word.unwrap().word);
                    self.sandwich.ingredients.push(ingredient.clone());
                    let (word, entry) = dict.first_word_in_class(WordFunction::Affirmation);
                    display
                        .send(Render {
                            ingredients: self.sandwich.ingredients.clone(),
                            // TODO Produce English subtitles.
                            subtitles: entry.definition.clone(),
                        })
                        .unwrap();
                    return (word.into(), None, None);
                }
            }
            Some(WordFunction::Greeting) => {
                // Represents showing the sandwich to the client.
                println!("{:?}", self.sandwich);
                let (word, entry) = dict.first_word_in_class(WordFunction::Greeting);
                display
                    .send(Render {
                        ingredients: self.sandwich.ingredients.clone(),
                        subtitles: entry.definition.clone(),
                    })
                    .unwrap();
                // End the conversation.
                return (
                    word.into(),
                    Some(self.sandwich.clone()),
                    Some(Box::new(Idle)),
                );
            }
            _ => (),
        }

        let (word, entry) = dict.first_word_in_class(WordFunction::Negation);
        display
            .send(Render {
                ingredients: self.sandwich.ingredients.clone(),
                subtitles: entry.definition.clone(),
            })
            .unwrap();

        (word.into(), None, None)
    }
}
