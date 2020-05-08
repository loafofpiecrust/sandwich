use crate::behavior::Behavior;
use crate::grammar::WordFunction;
use crate::sandwich::{Ingredient, Sandwich};
use crate::{behavior, grammar, sandwich};
use itertools::zip;
use seqalign::{measures::LevenshteinDamerau, Align};

#[derive(Default)]
pub struct Client {
    pub context: grammar::Context,
    behaviors: Vec<Box<dyn Behavior>>,
    pub sandwich: Option<Sandwich>,
    ingredients_left: Vec<Ingredient>,
}
impl Client {
    pub fn invent_sandwich(&self) -> Sandwich {
        Sandwich::random(&self.context.dictionary.ingredients, 5)
    }
    pub fn add_behavior(&mut self, b: Box<dyn Behavior>) {
        self.behaviors.push(b);
    }
    pub fn start_order(&mut self, other: &mut Client) {
        let sammich = self.invent_sandwich();
        self.ingredients_left = sammich.ingredients.clone();
        self.sandwich = Some(sammich);
        self.greet(other);
        for b in &self.behaviors {
            b.start();
        }
    }
    pub fn end_order(&mut self, other: &mut Client) -> f64 {
        for b in &self.behaviors {
            b.end();
        }
        let score = self
            .greet(other)
            .map(|x| self.judge_sandwich(&x))
            .unwrap_or(0.0);
        println!("sandwich score: {}", score);
        self.sandwich = None;
        score
    }
    fn greet(&self, other: &mut Client) -> Option<Sandwich> {
        let hello = self
            .context
            .dictionary
            .first_word_in_class(WordFunction::Greeting);
        let greeting = grammar::phrase(hello.as_bytes());
        if let Ok((_, phrase)) = greeting {
            let parsed = grammar::annotate(&phrase, &self.context);
            let (resp, sandwich) = other.context.respond(&parsed);
            println!("{}", resp);
            sandwich
        } else {
            None
        }
    }
    pub fn next_phrase(&mut self) -> Option<String> {
        let sandwich = self.sandwich.as_ref().unwrap();

        if self.ingredients_left.is_empty() {
            return None;
        }

        let mut next_ingredient = Some(self.ingredients_left.remove(0));
        // Allow behavior to change what the next ingredient might be.
        for b in &mut self.behaviors {
            next_ingredient = b.next_ingredient(&mut self.ingredients_left, next_ingredient);
        }

        let obj = next_ingredient
            .map(|ingredient| {
                let word = self
                    .context
                    .dictionary
                    .ingredients
                    .to_word(&ingredient, "".into());
                word
            })
            .flatten();

        obj.map(|obj| {
            let verb = self
                .context
                .dictionary
                .first_word_in_class(WordFunction::Desire);
            format!("{} {}", obj, verb)
        })
    }

    /// Returns a score for the match between the sandwich we wanted and the sandwich we got.
    /// TODO A low enough score may warrant revisions, depending on how shy this client is.
    pub fn judge_sandwich(&self, result: &Sandwich) -> f64 {
        // For now, just count the number of ingredients that match.
        // TODO Count the number of matching *morphemes*.
        if let Some(sandwich) = &self.sandwich {
            // Number of correct ingredients we did ask for.
            let measure = LevenshteinDamerau::new(1, 1, 1, 1);
            let alignment = measure.align(&result.ingredients, &sandwich.ingredients);
            1.0 / (alignment.distance() + 1) as f64
        } else {
            0.0
        }
    }

    pub fn respond(&mut self, prompt: &str) -> (String, Option<Sandwich>) {
        if let Ok((_, phrase)) = grammar::phrase(prompt.as_bytes()) {
            let annotated = grammar::annotate(&phrase, &self.context);
            self.context.respond(&annotated)
        } else {
            // TODO use dictionary for all responses.
            (
                self.context
                    .dictionary
                    .first_word_in_class(WordFunction::Negation)
                    .into(),
                None,
            )
        }
    }
}
