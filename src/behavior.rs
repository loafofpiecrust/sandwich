use crate::sandwich::Ingredient;
use crate::Client;
use rand;
use rand::prelude::*;

pub trait Behavior {
    fn start(&self);
    fn end(&self);
    fn next_ingredient<'a>(
        &self,
        ingredients_left: &'a [&Ingredient],
        pick: &'a Ingredient,
    ) -> &'a Ingredient;
}

#[derive(Clone, Default)]
pub struct Forgetful;
impl Behavior for Forgetful {
    fn start(&self) {}
    fn end(&self) {}
    fn next_ingredient<'a>(
        &self,
        ingredients_left: &'a [&Ingredient],
        pick: &'a Ingredient,
    ) -> &'a Ingredient {
        let mut rng = thread_rng();
        // TODO Chance to remember a forgotten ingredient.
        if ingredients_left.len() > 1 && rng.gen_bool(0.05) {
            &ingredients_left[1]
        } else {
            pick
        }
    }
}
