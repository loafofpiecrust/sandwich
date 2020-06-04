use crate::sandwich::Ingredient;
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
    pub order_sensitivity: f64,
}
impl Personality {
    pub fn new() -> Self {
        Self {
            laziness: 0.5,
            forgetfulness: 0.5,
            politeness: 0.5,
            shyness: 0.1,
            spite: 0.0,
            order_sensitivity: 1.0,
        }
    }
}
