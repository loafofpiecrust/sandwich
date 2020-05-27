use crate::display::Render;
use crate::grammar::*;
use crate::{behavior::Encoder, client::Language, sandwich::Sandwich};

pub trait State {
    // TODO Make this `respond(self) -> Box<dyn State>` so we can move data at the end of
    // a state.
    fn respond(
        &mut self,
        input: &PhraseNode,
        lang: &Language,
        encoder: &mut dyn Encoder,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>);
}

/// Initial state when not conversing with any other robot
pub struct Idle;
impl State for Idle {
    fn respond(
        &mut self,
        input: &PhraseNode,
        lang: &Language,
        _encoder: &mut dyn Encoder,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>) {
        // Only respond if being properly greeted.
        if let Some(WordFunction::Greeting) = input.main_verb().and_then(|v| v.definition()) {
            (
                lang.dictionary
                    .first_word_in_class(WordFunction::Greeting)
                    .0
                    .into(),
                None,
                Some(Box::new(SandwichOrder::new())),
            )
        } else {
            (
                lang.dictionary
                    .first_word_in_class(WordFunction::Negation)
                    .0
                    .into(),
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
        lang: &Language,
        encoder: &mut dyn Encoder,
    ) -> (String, Option<Sandwich>, Option<Box<dyn State>>) {
        let verb = input
            .main_verb()
            .and_then(|x| x.entry.clone())
            .unwrap()
            .function;
        let (word, sammich) = match verb {
            WordFunction::Greeting => (WordFunction::Greeting, Some(&self.sandwich)),
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
                ingredients: sammich.map(|s| s.ingredients.clone()).unwrap_or_default(),
                subtitles: entry.definition.clone(),
            })
            .unwrap();

        (word.into(), None, None)
    }
}
