use crate::grammar::*;
use itertools::Itertools;
use rand::prelude::*;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::fs::File;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ingredient {
    pub name: String,
    morpheme: String,
    children: Option<Vec<Ingredient>>,
}
impl Ingredient {
    pub fn all() -> Self {
        let file = File::open("ingredients.yml").unwrap();
        serde_yaml::from_reader(file).unwrap()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn includes(&self, other: &Ingredient) -> bool {
        self == other
            || self
                .children
                .as_ref()
                .map_or(false, |children| children.iter().any(|x| x.includes(other)))
    }

    pub fn random(&self) -> &Ingredient {
        if let Some(children) = &self.children {
            children
                .iter()
                // Inner ingredients can't be a base.
                .filter(|x| x.name != "base")
                .choose(&mut thread_rng())
                .unwrap()
                .random()
        } else {
            &self
        }
    }

    pub fn random_base(&self) -> (&Ingredient, &Ingredient) {
        self.children
            .as_ref()
            // Look for the "base" category.
            .and_then(|cats| cats.iter().find(|c| c.name == "base"))
            // Look through all the different bases.
            .and_then(|b| b.children.as_ref())
            // Pick a random one.
            .and_then(|c| c.choose(&mut thread_rng()))
            // Grab all the children of the base, which should be [bottom, top].
            .and_then(|b| b.children.as_ref())
            .and_then(|c| c.iter().tuples().next())
            .unwrap()
    }

    pub fn leaves(&self) -> Vec<(String, String)> {
        if let Some(children) = self.children.as_ref() {
            children
                .iter()
                .flat_map(|x| {
                    x.leaves()
                        .into_iter()
                        .map(|(n, x)| (n, format!("{}{}", self.morpheme, x)))
                })
                .collect()
        } else {
            vec![(self.name.clone(), self.morpheme.clone())]
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

pub const BG_COLORS: &[&str] = &["#00000000"];

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Sandwich {
    pub ingredients: Vec<Ingredient>,
    pub complete: bool,
    pub background_color: String,
}
impl Sandwich {
    pub fn new(ingredients: Vec<Ingredient>) -> Self {
        Self {
            ingredients,
            complete: false,
            background_color: BG_COLORS[0].into(),
        }
    }
    pub fn random(all_ingredients: &Ingredient, len: usize) -> Self {
        let mut rng = thread_rng();
        // Pick a base first, then the inside ingredients.
        let mut ingredients = Vec::new();
        let (bottom, top) = all_ingredients.random_base();
        ingredients.push(bottom.clone());
        ingredients.extend(
            (0..)
                .map(|_| all_ingredients.random().clone())
                // 20% chance for an ingredient to duplicate.
                .unique_by(|x| format!("{}{}", x.name, rng.gen_bool(0.8)))
                .take(len),
        );
        ingredients.push(top.clone());
        Self {
            ingredients,
            complete: true,
            background_color: BG_COLORS.choose(&mut rng).unwrap().to_string(),
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
