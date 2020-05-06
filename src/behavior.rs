use crate::sandwich::Ingredient;
use crate::Client;
use rand;
use rand::prelude::*;

pub trait Behavior {
    fn start(&self);
    fn end(&self);
    fn next_ingredient(
        &mut self,
        ingredients_left: &mut Vec<Ingredient>,
        pick: Option<Ingredient>,
    ) -> Option<Ingredient>;
}

#[derive(Clone, Default)]
pub struct Forgetful {
    forgotten: Vec<Ingredient>,
}
impl Behavior for Forgetful {
    fn start(&self) {}
    fn end(&self) {}
    fn next_ingredient(
        &mut self,
        ingredients_left: &mut Vec<Ingredient>,
        pick: Option<Ingredient>,
    ) -> Option<Ingredient> {
        let mut rng = thread_rng();
        // TODO Chance to remember a forgotten ingredient.
        if rng.gen_bool(0.05) && !self.forgotten.is_empty() {
            Some(self.forgotten.remove(0))
        } else if ingredients_left.len() > 1 && rng.gen_bool(0.10) {
            self.forgotten.push(ingredients_left.remove(0));
            Some(ingredients_left.remove(0))
        } else {
            pick
        }
    }
}
