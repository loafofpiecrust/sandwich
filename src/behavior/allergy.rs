use super::Behavior;
use crate::sandwich::{Ingredient, Sandwich};

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
    fn react(&self, sandwich: &Sandwich) {}
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
