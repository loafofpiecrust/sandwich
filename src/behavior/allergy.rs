use super::Behavior;
use crate::{
    client::Language,
    grammar::WordFunction,
    sandwich::{Ingredient, Sandwich},
};

pub enum Severity {
    Minor,
    Mild,
    Severe,
    Fatal,
}
impl Severity {
    pub fn reaction_chance(&self) -> f64 {
        use Severity::*;
        match self {
            Minor => 0.2,
            Mild => 0.4,
            Severe => 0.6,
            Fatal => 1.0,
        }
    }
}

/// Behavior provides motivation for a change to the sandwich, so
/// Sandwich (current) => Sandwich (desired)
/// Encoder provides a method for expressing a patch to the Sandwich.
/// This may mean adding on top (DesireEncoder), adding in the middle (RelativeEncoder),
/// removing (NegationEncoder). Each level of encoder needs to be able to affect but also
/// invoke all layers contained within. NegationEncoder, for example, needs to turn all
/// inner additions into removals.

/// TODO Define severity breakpoints
pub struct Allergy {
    severity: Severity,
    ingredient: &'static Ingredient,
}
impl Allergy {
    pub fn new(severity: Severity, ingredient: &'static Ingredient) -> Self {
        Self {
            severity,
            ingredient,
        }
    }
    fn is_allergic(&self, ingredient: &Ingredient) -> bool {
        self.ingredient.includes(ingredient)
    }
    fn react(&self, sandwich: &Sandwich, lang: &Language) -> Option<String> {
        // Need access to the dictionary.
        let allergic = sandwich
            .ingredients
            .iter()
            .filter(|x| self.is_allergic(x))
            .next();
        allergic.map(|bad_item| {
            // Say "remove this ingredient"
            let verb = lang.dictionary.word_for_def(WordFunction::Desire);
            let neg = lang.dictionary.word_for_def(WordFunction::Negation);
            let ingr = lang.dictionary.ingredients.to_word(bad_item, String::new());
            format!("{} {} {}", ingr.unwrap(), verb.0, neg.0)
        })
    }
}
impl Behavior for Allergy {
    fn start(&self) {
        if self.severity.reaction_chance() > 0.5 {
            // Tell the other machine that we're allergic.
        }
    }
    fn end(&self) {}
    fn next_ingredient(&mut self, sandwich: &Sandwich, pick: Option<usize>) -> Option<usize> {
        pick
    }
}
