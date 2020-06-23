use crate::{
    display::{setup_display, Display, Render, RenderSender},
    grammar::{
        AnnotatedPhrase, Dictionary, DictionaryEntry, MeaningCloud, Weights, WordFunction,
        DEFAULT_WORD_MAP,
    },
    sandwich::{Ingredient, Sandwich, BG_COLORS},
};
use itertools::Itertools;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, time::Duration, time::Instant};

type Inventory = HashMap<String, usize>;

#[derive(Default)]
pub struct Language {
    // Weights for grammar rules!
    pub adverbs: i32,
    pub adverb_side: i32,
    pub adposition: i32,
    pub conjunction: i32,
    pub numbers: i32,
}
impl std::ops::Add for Language {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            adverbs: self.adverbs + rhs.adverbs,
            adposition: self.adposition + rhs.adposition,
            conjunction: self.conjunction + rhs.conjunction,
            numbers: self.numbers + rhs.numbers,
            adverb_side: self.adverb_side + rhs.adverb_side,
        }
    }
}

pub enum Event {
    LunchRush(Instant),
}
impl Event {
    pub fn duration(&self) -> Duration {
        use Event::*;
        match self {
            // Lunch rush lasts an hour.
            LunchRush(_) => Duration::from_secs(3600),
        }
    }
    pub fn stress(&self) -> f64 {
        use Event::*;
        match self {
            LunchRush(_) => 2.0,
        }
    }
    pub fn is_over(&self) -> bool {
        match self {
            Event::LunchRush(started) => Instant::now().duration_since(*started) > self.duration(),
        }
    }
}

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
    pub allergies: Vec<Preference>,
    pub preferences: Vec<Preference>,
    // Weights for grammar rules!
    pub adverbs: f64,
    pub adverb_side: f64,
    pub adposition: f64,
    pub conjunction: f64,
    pub numbers: f64,
    /// Maps ingredient names to their inventory count.
    pub inventory: Inventory,
    pub history: Vec<Sandwich>,
    pub cloud: MeaningCloud,
    #[serde(skip)]
    pub event: Option<Event>,
    #[serde(skip, default = "Dictionary::new")]
    pub dictionary: Dictionary,
    #[serde(skip, default = "setup_display")]
    pub display: Display,
    #[serde(skip)]
    pub last_lex: Option<AnnotatedPhrase>,
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
            allergies: vec![Preference {
                severity: 0.6,
                ingredient: dictionary.ingredients.random().clone(),
            }],
            // TODO Add preferences for other ingredients starting at zero??
            preferences: vec![Preference {
                severity: 0.8,
                ingredient: dictionary.ingredients.random().clone(),
            }],
            // Fill our cloud with equal weights on every definition for all words.
            cloud: Default::default(),
            // Grammar rule weights
            adverbs: 0.1,
            adverb_side: 0.95,
            adposition: 0.1,
            conjunction: 0.1,
            numbers: 0.1,
            inventory: Self::default_inventory(&dictionary),
            dictionary,
            last_lex: None,
            history: Vec::new(),
            event: None,
        }
    }

    pub fn stress(&self) -> f64 {
        self.event.as_ref().map(|e| e.stress()).unwrap_or(1.0)
    }

    fn default_inventory(dict: &Dictionary) -> Inventory {
        const DEFAULT_INGREDIENT_COUNT: usize = 20;
        // Grab all the bottom-level ingredients.
        let ingredients = dict.ingredients.leaves();
        ingredients
            .iter()
            // Provide a few for each one.
            .map(|(name, _)| (name.clone(), DEFAULT_INGREDIENT_COUNT))
            .collect()
    }

    pub fn reset_inventory(&mut self) {
        self.inventory = Self::default_inventory(&self.dictionary);
    }

    pub fn total_inventory_count(&self) -> usize {
        self.inventory.iter().map(|(_, count)| count).sum()
    }

    /// Retrieve the meaning distribution for a particular word.
    /// If we find no matching entry, create one with equal weights for all definitions.
    pub fn get_cloud_entry(&self, key: &str) -> &Weights<DictionaryEntry> {
        self.cloud.get(key).unwrap_or(&DEFAULT_WORD_MAP)
    }

    pub fn improve_match(&mut self, key: &str, def: WordFunction) {
        let weights = self
            .cloud
            .entry(key.to_owned())
            .or_insert(DEFAULT_WORD_MAP.clone());
        // Find the weight matching the dictionary entry we used in this lex.
        if let Some(entry) = weights.iter_mut().find(|(e, _)| e.function == def) {
            // Increase the score of this one!
            // TODO Use some gradient so that an early success counts for a lot?
            entry.1 += 1;
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
    pub fn apply_upgrade(&mut self, lang: Language) {
        Self::upgrade_skill(&mut self.adposition, lang.adposition as f64);
        Self::upgrade_skill(&mut self.numbers, lang.numbers as f64);
        Self::upgrade_skill(&mut self.adverbs, lang.adverbs as f64);
        Self::upgrade_skill(&mut self.conjunction, lang.conjunction as f64);
        Self::upgrade_skill(&mut self.adverb_side, lang.adverb_side as f64);
    }
    pub fn upgrade_skill(x: &mut f64, mult: f64) {
        let orig = *x;
        *x = (*x + ((*x * 100.0).ln()) / 100.0 * mult).min(1.0);
        println!("Upgraded language skill from {} => {}", orig, *x);
    }
    pub fn render(&self, state: Render) -> anyhow::Result<()> {
        Ok(self.display.render.send(state)?)
    }
    pub fn increase_preference(&mut self, name: &str) {
        // If we already have some preference for this ingredient, increase its severity.
        if let Some(pref) = self
            .preferences
            .iter_mut()
            .find(|x| &x.ingredient.name == name)
        {
            Self::upgrade_skill(&mut pref.severity, 1.0);
        }
        // Otherwise, add a new preference with the base severity.
        self.preferences.push(Preference {
            ingredient: self.dictionary.ingredients.from_def(name).unwrap().clone(),
            severity: 0.1,
        });
    }
    pub fn has_ingredient(&self, desired: &Ingredient) -> bool {
        *self.inventory.get(&desired.name).unwrap_or(&0) > 0
    }
    pub fn use_ingredient(&mut self, used: &Ingredient) {
        let name = used.name.clone();
        let prev = *self.inventory.get(&name).unwrap_or(&0);
        if prev > 0 {
            self.inventory.insert(name, prev - 1);
        }
    }
    pub fn gen_sandwich(&self, mut len: usize) -> Sandwich {
        let mut rng = thread_rng();
        let mut ingredients = Vec::new();
        // Make sandwich sizes more varied.
        let len = rng.gen_range(len / 2, len * 2);
        // Pick a base first, then the inside ingredients.
        let (bottom, top) = self.dictionary.ingredients.random_base();
        ingredients.push(bottom.clone());
        // Choose ingredients based on our current preferences.
        // Preferences and allergies could override each other applying to the
        // same ingredient.
        for fav in &self.preferences {
            if rng.gen_bool((fav.severity * self.stress()).min(0.9)) {
                ingredients.push(fav.ingredient.clone());
                len -= 1;
            }
        }
        ingredients.extend(
            (0..)
                .map(|_| self.dictionary.ingredients.random().clone())
                // 50% chance for a duplicate ingredient to stay.
                .unique_by(|x| format!("{}{}", x.name, rng.gen_bool(0.3)))
                .take(len),
        );
        ingredients.push(top.clone());
        Sandwich {
            ingredients,
            complete: true,
            background_color: BG_COLORS.choose(&mut rng).unwrap().to_string(),
        }
    }

    /// Keep just ten of our last sandwiches in a stack.
    /// We might use those memories later to shape our choices.
    pub fn eat(&mut self, sandwich: Sandwich) {
        self.history.insert(0, sandwich);
        self.history.truncate(10);
    }
}

#[derive(Serialize, Deserialize)]
pub struct Preference {
    pub ingredient: Ingredient,
    pub severity: f64,
}
