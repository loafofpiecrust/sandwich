use crate::{client::Language, sandwich::Ingredient};

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
    pub spontaneity: f64,
    pub order_sensitivity: f64,
    pub allergies: Vec<Allergy>,
    pub favorites: Vec<Allergy>,
}
impl Personality {
    pub fn new(lang: &Language) -> Self {
        Self {
            laziness: 0.5,
            forgetfulness: 0.1,
            politeness: 0.5,
            shyness: 0.1,
            spite: 0.0,
            order_sensitivity: 1.0,
            spontaneity: 0.05,
            allergies: vec![Allergy {
                severity: 0.5,
                ingredient: lang.dictionary.ingredients.random().clone(),
            }],
            // TODO Generate some favorites!
            favorites: Vec::new(),
        }
    }
}

pub struct Allergy {
    pub ingredient: Ingredient,
    pub severity: f64,
}
