use crate::grammar::*;
use itertools::Itertools;
use rand::prelude::*;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::fs::File;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ingredient {
    name: String,
    morpheme: String,
    children: Option<Vec<Ingredient>>,
}
impl Ingredient {
    pub fn all() -> Self {
        let file = File::open("ingredients.yml").unwrap();
        serde_yaml::from_reader(file).unwrap()
    }

    pub fn random(&self) -> &Ingredient {
        if let Some(children) = &self.children {
            children.choose(&mut thread_rng()).unwrap().random()
        } else {
            &self
        }
    }

    /// Retrieve the ingredient that corresponds to the given word, based on the
    /// single-syllable morphemes contained within.
    /// Assumes that syllables and morphemes are always one-to-one.
    pub fn from_word(&self, word: &Word) -> &Ingredient {
        let mut current = self;
        for syllable in word.0.iter().skip(1) {
            let text = format!("{}", syllable);
            if let Some(children) = &current.children {
                for child in children {
                    if text == child.morpheme {
                        current = child;
                    }
                }
            }
        }
        current
    }

    pub fn to_word(&self, ingredient: &Ingredient, word_so_far: String) -> Option<String> {
        if self.name == ingredient.name {
            return Some(format!("{}{}", word_so_far, self.morpheme));
        }
        if let Some(children) = &self.children {
            for child in children {
                if let Some(dfs) =
                    child.to_word(ingredient, format!("{}{}", word_so_far, self.morpheme))
                {
                    return Some(dfs);
                }
            }
        }
        None
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Sandwich {
    pub ingredients: Vec<Ingredient>,
}
impl Sandwich {
    pub fn new(ingredients: Vec<Ingredient>) -> Self {
        Self { ingredients }
    }
    pub fn random(all_ingredients: &Ingredient, len: usize) -> Self {
        Self {
            ingredients: (0..)
                .map(|_| all_ingredients.random().clone())
                .unique()
                .take(len)
                .collect(),
        }
    }
    pub fn to_words(&self, dictionary: &Dictionary) -> Vec<String> {
        self.ingredients
            .iter()
            .map(|x| {
                dictionary
                    .ingredients
                    .to_word(x, "".into())
                    .unwrap_or_default()
            })
            .collect()
    }
}

pub trait SandwichRule {
    /// Whether the given ingredient is allowed to be next on the given sandwich so far.
    fn ingredient_allowed(&self, sandwich: &Sandwich, ingredient: &Ingredient);
}
