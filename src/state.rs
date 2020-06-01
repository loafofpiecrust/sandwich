use crate::display::Render;
use crate::grammar::*;
use crate::{
    behavior::{Behavior, Behaviors, Encoder, PositionedIngredient},
    client::Language,
    sandwich::{Ingredient, Sandwich},
};

pub trait State: std::fmt::Debug {
    // TODO Make this `respond(self) -> Box<dyn State>` so we can move data at the end of
    // a state.
    fn respond(
        &mut self,
        input: &PhraseNode,
        lang: &Language,
        encoder: &mut dyn Encoder,
        behavior: &mut Behaviors,
    ) -> (Option<String>, Option<Sandwich>, Option<Box<dyn State>>);
}

/// Initial state when not conversing with any other robot
#[derive(Debug)]
pub struct Idle;
impl State for Idle {
    fn respond(
        &mut self,
        input: &PhraseNode,
        lang: &Language,
        _encoder: &mut dyn Encoder,
        behavior: &mut Behaviors,
    ) -> (Option<String>, Option<Sandwich>, Option<Box<dyn State>>) {
        // Only respond if being properly greeted.
        // If the other machine says "I want sandwich", then we're making one.
        match input.main_verb().and_then(|v| v.definition()) {
            Some(WordFunction::Desire) => {
                if let Some(WordFunction::Sandwich) = input.object().and_then(|s| s.definition()) {
                    return (
                        Some(
                            lang.dictionary
                                .first_word_in_class(WordFunction::Greeting)
                                .0
                                .into(),
                        ),
                        None,
                        Some(Box::new(MakingSandwich::new())),
                    );
                }
            }
            Some(WordFunction::Greeting) => {
                return (
                    Some("ku nu".into()),
                    None,
                    Some(Box::new(OrderingSandwich::new(
                        &lang.dictionary.ingredients,
                    ))),
                )
            }
            _ => (),
        }

        (
            Some(
                lang.dictionary
                    .first_word_in_class(WordFunction::Negation)
                    .0
                    .into(),
            ),
            None,
            None,
        )
    }
}

/// Receiving an order for a sandwich
#[derive(Debug)]
pub struct MakingSandwich {
    sandwich: Sandwich,
}
impl MakingSandwich {
    fn new() -> Self {
        Self {
            sandwich: Sandwich::default(),
        }
    }
}
impl State for MakingSandwich {
    fn respond(
        &mut self,
        input: &PhraseNode,
        lang: &Language,
        encoder: &mut dyn Encoder,
        behavior: &mut Behaviors,
    ) -> (Option<String>, Option<Sandwich>, Option<Box<dyn State>>) {
        let verb = input
            .main_verb()
            .and_then(|x| x.entry.clone())
            .unwrap()
            .function;
        let (word, sammich) = match verb {
            WordFunction::Greeting => (WordFunction::Greeting, Some(self.sandwich.clone())),
            WordFunction::Desire => {
                encoder.decode(input, &mut self.sandwich, lang);
                // TODO Say "no" or more if decode fails.
                (WordFunction::Affirmation, None)
            }
            _ => (WordFunction::Negation, None),
        };

        let (word, entry) = lang.dictionary.first_word_in_class(word);
        lang.display
            .send(Render {
                ingredients: self.sandwich.ingredients.clone(),
                subtitles: entry.definition.clone(),
            })
            .unwrap();

        (Some(word.into()), sammich, None)
    }
}

#[derive(Debug)]
pub struct OrderingSandwich {
    sandwich: Sandwich,
    next_index: usize,
    history: Vec<usize>,
}
impl OrderingSandwich {
    pub fn new(all_ingredients: &Ingredient) -> Self {
        Self {
            sandwich: Sandwich::random(all_ingredients, 5),
            next_index: 0,
            history: Vec::new(),
        }
    }
}
impl State for OrderingSandwich {
    fn respond(
        &mut self,
        input: &PhraseNode,
        lang: &Language,
        encoder: &mut dyn Encoder,
        behavior: &mut Behaviors,
    ) -> (Option<String>, Option<Sandwich>, Option<Box<dyn State>>) {
        let mut next_ingredient = Some(self.next_index);
        // Allow behavior to change what the next ingredient might be.
        for b in behavior {
            next_ingredient = b.next_ingredient(&self.sandwich, next_ingredient);
        }

        println!("next ingredient: {:?}", next_ingredient);

        let s = if let Some(idx) = next_ingredient {
            if idx >= self.sandwich.ingredients.len() {
                None
            } else {
                let result = Some(encoder.encode(
                    lang,
                    PositionedIngredient {
                        sandwich: &self.sandwich,
                        index: idx,
                        history: &self.history[..],
                    },
                ));
                self.history.push(idx);
                self.next_index = self.history.iter().max().unwrap_or(&0) + 1;
                result
            }
        } else {
            None
        };

        (s, None, None)
    }
}
