pub struct Personality {
    pub laziness: f64,
    pub forgetfulness: f64,
    pub politeness: f64,
    pub shyness: f64,
}
impl Personality {
    pub fn new() -> Self {
        Self {
            laziness: 0.5,
            forgetfulness: 0.5,
            politeness: 0.5,
            shyness: 0.1,
        }
    }
}
