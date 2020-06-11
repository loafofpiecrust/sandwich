use crate::{
    display::{setup_display, Render, RenderSender},
    grammar::Dictionary,
    sandwich::Ingredient,
};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::prelude::*};

#[derive(Serialize, Deserialize)]
pub struct Personality {
    /// Likeliness to make mistakes building an order, to fail to remove allergens.
    pub laziness: f64,
    /// How likely are you to forget an ingredient you wanted on your sandwich?
    pub forgetfulness: f64,
    /// Likeliness to use polite modifiers.
    pub politeness: f64,
    /// Likeliness to correct mistakes or bring up dietary preferences.
    pub shyness: f64,
    /// Others being polite lowers your spite, non-polite interactions raise spite.
    /// Once you reach a high spite threshold, mess up orders on purpose.
    pub spite: f64,
    pub planned: f64,
    pub spontaneity: f64,
    pub order_sensitivity: f64,
    pub allergies: Vec<Allergy>,
    pub favorites: Vec<Allergy>,
    // Weights for grammar rules!
    pub adverbs: f64,
    pub adposition: f64,
    pub conjunction: f64,
    pub numbers: f64,
    #[serde(skip, default = "Dictionary::new")]
    pub dictionary: Dictionary,
    #[serde(skip, default = "setup_display")]
    pub display: RenderSender,
}
impl Personality {
    pub fn new() -> Self {
        let dictionary = Dictionary::new();
        Self {
            display: setup_display(),
            planned: 0.8,
            laziness: 0.5,
            forgetfulness: 0.1,
            politeness: 0.3,
            shyness: 0.1,
            spite: 0.0,
            order_sensitivity: 1.0,
            spontaneity: 0.1,
            allergies: vec![Allergy {
                severity: 0.6,
                ingredient: dictionary.ingredients.random().clone(),
            }],
            favorites: vec![Allergy {
                severity: 0.8,
                ingredient: dictionary.ingredients.random().clone(),
            }],
            // Grammar rule weights
            adverbs: 0.1,
            adposition: 0.1,
            conjunction: 0.1,
            numbers: 0.1,
            dictionary,
        }
    }
    pub fn load() -> anyhow::Result<Self> {
        let f = File::open("personality.yaml")?;
        Ok(serde_yaml::from_reader(&f)?)
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let mut f = File::create("personality.yaml")?;
        Ok(serde_yaml::to_writer(f, self)?)
    }
    pub fn degrade_language_skills(&mut self) {
        let factor = 6.0;
        let deg = |x: &mut f64| {
            *x = (*x - ((*x * 100.0).ln()) / (factor * 100.0)).max(0.0);
        };
        deg(&mut self.adverbs);
        deg(&mut self.adposition);
        deg(&mut self.conjunction);
        deg(&mut self.numbers)
    }
    pub fn upgrade_skill(x: &mut f64) {
        let orig = *x;
        *x = (*x + ((*x * 100.0).ln()) / 100.0).min(1.0);
        println!("Upgraded language skill from {} => {}", orig, *x);
    }
    pub fn render(&self, state: Render) -> anyhow::Result<()> {
        Ok(self.display.send(state)?)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Allergy {
    pub ingredient: Ingredient,
    pub severity: f64,
}
