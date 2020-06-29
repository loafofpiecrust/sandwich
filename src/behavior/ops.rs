//! A machine ordering a sandwich first comes up with what they want.
//! Let's assume for now that there are no presets, they come up with their own recipe.
//!
//! Sandwich struct represents the current state of the conversation, which may soon
//! contain more than simply a list of ingredients.
//!
//! A: I want a sandwich => B: okay
//! A: Add(WheatBread, Relative::End)
//!    => encodes as "I want wheat bread"
//!    => B receives "I want wheat bread"
//!    => decodes it: op = Add(WheatBread, Relative::End)
//!    => sandwich = op.apply(sandwich)
//!
//! The negation particle reverses the operation if found.
//! Once we implement nested operators, position and order of negation may matter.
//! "no avocado want" => Add(Avocado, Relative::End).reverse() => Remove(Avocado)
//!
//! Basically, a VerbPhrase maps to an Operation that we can apply.
//! Modifiers atop that verb phrase may either nest the operation, reverse it, or modify
//! it in other arbitrary ways.

use crate::{
    behavior::{Language, Personality},
    grammar::{
        self, AnnotatedPhrase, AnnotatedWord, DictionaryEntry, WordFunction, DEFAULT_WORD_MAP,
    },
    sandwich::{Ingredient, Sandwich},
};
use async_std::net::TcpStream;
use async_std::prelude::*;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;

/// An operation makes some change to a sandwich based on its internal structure
/// and the [Personality] passed to it.
pub trait Operation: std::fmt::Debug {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich;
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>>;
    fn reverse(&self) -> Box<dyn Operation>;
    fn question(&self) -> Box<dyn Operation>;
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase;
    fn is_persistent(&self) -> bool;
    fn skills(&self) -> Language;
}

/// Add an ingredient to a sandwich, at the very end or relative to another ingredient.
#[derive(Debug)]
pub struct Add(pub Ingredient, pub Relative);
impl Operation for Add {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        // FIXME
        if !personality.has_ingredient(&self.0) {
            return sandwich;
        }

        let mut ingr = sandwich.ingredients;
        let idx = match &self.1 {
            Relative::Before(other) => ingr.iter().position(|x| x.name == other.name),
            Relative::After(other) => ingr
                .iter()
                .position(|x| x.name == other.name)
                .map(|x| x + 1),
            Relative::Top => Some(ingr.len()),
        };
        if let Some(idx) = idx {
            ingr.insert(idx, self.0.clone());
            personality.use_ingredient(&self.0);
        }
        // Personality::upgrade_skill(&mut personality.spite);
        // personality.spite += 0.05;
        Sandwich {
            ingredients: ingr,
            ..sandwich
        }
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Remove(self.0.clone()))
    }
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase {
        // Encode prepositional phrase.
        // TODO Use language weight for whether to actually use the adposition.
        let mut prep = match &self.1 {
            Relative::Before(other) => {
                let p = lang.dictionary.annotated_word_for_def(WordFunction::Before);
                let n = lang.dictionary.ingredients.to_annotated_word(&other);
                vec![n, p]
            }
            Relative::After(other) => {
                let p = lang.dictionary.annotated_word_for_def(WordFunction::After);
                let n = lang.dictionary.ingredients.to_annotated_word(&other);
                vec![n, p]
            }
            Relative::Top => vec![],
        };

        // Get the word for our verb and ingredient.
        let want = lang.dictionary.annotated_word_for_def(WordFunction::Desire);
        let ingr = lang.dictionary.ingredients.to_annotated_word(&self.0);
        prep.push(ingr);
        prep.push(want);
        prep
    }
    fn is_persistent(&self) -> bool {
        false
    }
    fn skills(&self) -> Language {
        Language {
            adposition: if self.1 == Relative::Top { 0 } else { 1 },
            ..Default::default()
        }
    }
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>> {
        // Never add the requested ingredient if we don't have any more.
        if !personality.has_ingredient(&self.0) {
            Some(Box::new(Remove(self.0.clone())))
        } else {
            None
        }
    }
    fn question(&self) -> Box<dyn Operation> {
        Box::new(CheckFor(self.0.clone()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Relative {
    Before(Ingredient),
    After(Ingredient),
    Top,
}
impl Relative {
    pub fn from_def(def: WordFunction, ingredient: Ingredient) -> Self {
        match def {
            WordFunction::Before => Relative::Before(ingredient),
            WordFunction::After => Relative::After(ingredient),
            _ => Relative::Top,
        }
    }
}

/// Remove the given ingredient from a sandwich.
#[derive(Debug)]
pub struct Remove(pub Ingredient);
impl Operation for Remove {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        let mut ingredients = sandwich.ingredients;
        if let Some(idx) = ingredients.iter().position(|x| x.name == self.0.name) {
            ingredients.remove(idx);
        }
        // Ingredient removal raises spite!
        // Personality::upgrade_skill(&mut personality.spite);
        Sandwich {
            ingredients,
            ..sandwich
        }
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Add(self.0.clone(), Relative::Top))
    }
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase {
        let mut phrase = self.reverse().encode(lang);
        let neg = lang
            .dictionary
            .annotated_word_for_def(WordFunction::Negation);
        phrase.insert(0, neg);
        phrase
    }
    fn is_persistent(&self) -> bool {
        false
    }
    fn skills(&self) -> Language {
        Language {
            adverbs: 1,
            ..Default::default()
        }
    }
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>> {
        None
    }
    fn question(&self) -> Box<dyn Operation> {
        todo!()
    }
}

#[derive(Debug)]
pub struct RemoveAll(pub Ingredient);
impl Operation for RemoveAll {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        Sandwich {
            ingredients: sandwich
                .ingredients
                .into_iter()
                .filter(|x| x.name != self.0.name)
                .collect(),
            ..sandwich
        }
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Ensure(self.0.clone()))
    }
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase {
        let all = lang.dictionary.annotated_word_for_def(WordFunction::Ever);
        let mut inner = Remove(self.0.clone()).encode(lang);
        inner.insert(0, all);
        inner
    }
    fn is_persistent(&self) -> bool {
        false
    }
    fn skills(&self) -> Language {
        Language {
            adverbs: 1,
            ..Default::default()
        }
    }
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>> {
        None
    }
    fn question(&self) -> Box<dyn Operation> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct Finish;
impl Operation for Finish {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        Sandwich {
            complete: true,
            ..sandwich
        }
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(self.clone())
    }
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase {
        let bye = lang
            .dictionary
            .annotated_word_for_def(WordFunction::Greeting);
        vec![bye]
    }
    fn is_persistent(&self) -> bool {
        false
    }
    fn skills(&self) -> Language {
        Default::default()
    }
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>> {
        None
    }
    fn question(&self) -> Box<dyn Operation> {
        todo!()
    }
}

/// Applies an operation on a sandwich multiple times.
#[derive(Debug)]
pub struct Repeat(pub u32, pub Box<dyn Operation>);
impl Operation for Repeat {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        let mut sandwich = sandwich;
        for _ in 0..self.0 {
            sandwich = self.1.apply(sandwich, personality);
        }
        sandwich
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Self(self.0, self.1.reverse()))
    }
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase {
        let mut phrase = self.1.encode(lang);
        let num = lang.dictionary.annotated_word_for_num(self.0);
        phrase.insert(0, num);
        phrase
    }
    fn is_persistent(&self) -> bool {
        false
    }
    fn skills(&self) -> Language {
        self.1.skills()
            + Language {
                numbers: 1,
                ..Default::default()
            }
    }
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>> {
        None
    }
    fn question(&self) -> Box<dyn Operation> {
        todo!()
    }
}

/// Applies two operations sequentially on a sandwich.
#[derive(Debug)]
pub struct Compound(pub Box<dyn Operation>, pub Box<dyn Operation>);
impl Operation for Compound {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        // Apply the inner operations sequentially.
        self.1
            .apply(self.0.apply(sandwich, personality), personality)
    }
    // TODO This could also just reverse the order of it?
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Compound(self.0.reverse(), self.1.reverse()))
    }
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase {
        let mut phrase = self.0.encode(lang);
        let conj = lang.dictionary.annotated_word_for_def(WordFunction::And);
        // Conjunction goes between two sub-phrases.
        phrase.push(conj);
        phrase.append(&mut self.1.encode(lang));
        phrase
    }
    fn is_persistent(&self) -> bool {
        false
    }
    fn skills(&self) -> Language {
        self.0.skills()
            + self.1.skills()
            + Language {
                conjunction: 1,
                ..Default::default()
            }
    }
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>> {
        None
    }
    fn question(&self) -> Box<dyn Operation> {
        todo!()
    }
}

/// A no-op that exists only as a foil to [RemoveAll].
#[derive(Debug)]
pub struct Ensure(pub Ingredient);
impl Operation for Ensure {
    fn apply(&self, mut sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        sandwich.ensured.push(self.0.clone());
        sandwich
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Remove(self.0.clone()))
    }
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase {
        todo!()
    }
    fn is_persistent(&self) -> bool {
        false
    }
    fn skills(&self) -> Language {
        todo!()
    }
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>> {
        None
    }
    fn question(&self) -> Box<dyn Operation> {
        Box::new(CheckFor(self.0.clone()))
    }
}

/// Applies to (roughly) the duration of an order, and means this ingredient
/// should never be added to the sandwich. Expressed with allergen terminology.
#[derive(Debug)]
pub struct Persist(pub Box<dyn Operation>);
impl Operation for Persist {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        self.0.apply(sandwich, personality)
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Persist(self.0.reverse()))
    }
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase {
        let mut p = self.0.encode(lang);
        let ever = lang.dictionary.annotated_word_for_def(WordFunction::Ever);
        p.insert(0, ever);
        p
    }
    fn is_persistent(&self) -> bool {
        true
    }
    fn skills(&self) -> Language {
        self.0.skills()
    }
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>> {
        None
    }
    fn question(&self) -> Box<dyn Operation> {
        todo!()
    }
}

/// Affirms that the last operation was applied correctly.
#[derive(Debug)]
pub struct Affirm;
impl Operation for Affirm {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        // Update our meaning associations with the last lex that's now been
        // affirmed correct!
        if let Some(lex) = personality.last_lex.clone() {
            // For each word in the lex, update its' weight in the association table.
            println!("last lex: {:?}", lex);
            for w in &lex {
                if let Some(dict_entry) = w.entry.as_ref() {
                    let s = w.word.to_string();
                    personality.improve_match(&s, dict_entry.function);
                }
            }
        }

        sandwich
    }
    fn reverse(&self) -> Box<dyn Operation> {
        todo!("Add pure negation operation")
    }
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase {
        let w = lang
            .dictionary
            .annotated_word_for_def(WordFunction::Affirmation);
        vec![w]
    }
    fn is_persistent(&self) -> bool {
        false
    }
    fn skills(&self) -> Language {
        Default::default()
    }
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>> {
        None
    }
    fn question(&self) -> Box<dyn Operation> {
        todo!()
    }
}

/// Just a dummy to act as a foil for [Affirm].
#[derive(Debug)]
pub struct Negate;
impl Operation for Negate {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        // TODO Maybe do something here?
        sandwich
    }
    fn reverse(&self) -> Box<dyn Operation> {
        Box::new(Affirm)
    }
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase {
        let w = lang
            .dictionary
            .annotated_word_for_def(WordFunction::Negation);
        vec![w]
    }
    fn is_persistent(&self) -> bool {
        false
    }
    fn skills(&self) -> Language {
        Default::default()
    }
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>> {
        None
    }
    fn question(&self) -> Box<dyn Operation> {
        todo!()
    }
}

/// Negates the last operation we requested, operates practically as a negative
/// response to a question.
#[derive(Debug)]
pub struct NegateLast;
impl Operation for NegateLast {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        todo!()
    }
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>> {
        todo!()
    }
    fn reverse(&self) -> Box<dyn Operation> {
        todo!()
    }
    fn question(&self) -> Box<dyn Operation> {
        todo!()
    }
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase {
        todo!()
    }
    fn is_persistent(&self) -> bool {
        todo!()
    }
    fn skills(&self) -> Language {
        todo!()
    }
}

/// TODO Consider turning this into a generic question wrapper. Adds the
/// modifier, makes no application, but must send a response. Think on it.
#[derive(Debug)]
struct CheckFor(pub Ingredient);
impl Operation for CheckFor {
    fn apply(&self, sandwich: Sandwich, personality: &mut Personality) -> Sandwich {
        sandwich
    }
    fn reverse(&self) -> Box<dyn Operation> {
        todo!()
    }
    fn encode(&self, lang: &Personality) -> AnnotatedPhrase {
        let q = lang
            .dictionary
            .annotated_word_for_def(WordFunction::Question);
        let verb = lang.dictionary.annotated_word_for_def(WordFunction::Have);
        let n = lang.dictionary.ingredients.to_annotated_word(&self.0);
        vec![q, n, verb]
    }
    fn is_persistent(&self) -> bool {
        false
    }
    fn skills(&self) -> Language {
        Default::default()
    }
    fn respond(&self, personality: &Personality) -> Option<Box<dyn Operation>> {
        // If we have the asked for ingredient, respond positively.
        if !personality.has_ingredient(&self.0) {
            Some(Box::new(Remove(self.0.clone())))
        } else {
            Some(Box::new(Ensure(self.0.clone())))
        }
    }
    fn question(&self) -> Box<dyn Operation> {
        // A questioned question becomes a statement.
        Box::new(Add(self.0.clone(), Relative::Top))
    }
}

// #[derive(Debug)]
// pub struct ChangeBackground(pub String);
// impl Operation for ChangeBackground {
//     fn apply(&self, sandwich: Sandwich) -> Sandwich {
//         Sandwich {
//             background_color: self.0.clone(),
//             ..sandwich
//         }
//     }
//     fn reverse(&self) -> Box<dyn Operation> {
//         todo!()
//     }
//     fn encode(&self, lang: &Personality) -> String {

//         todo!()
//     }
// }

// pub struct Negate(Box<dyn Operation>);
// impl Operation for Negate {
//     fn apply(self, sandwich: Sandwich) -> Sandwich {
//         self.0.reverse().apply(sandwich)
//     }
//     fn reverse(self) -> Box<dyn Operation> {
//         self.0
//     }
// }

pub struct Order {
    pub desired: Sandwich,
    history: Vec<Box<dyn Operation>>,
    pub last_result: Option<Sandwich>,
    pub persistent_ops: Vec<Box<dyn Operation>>,
}
impl Order {
    pub fn new(lang: &Personality) -> Self {
        Self {
            history: Vec::new(),
            // TODO Pick a sandwich based on our personality.
            desired: lang.gen_sandwich(7),
            persistent_ops: Vec::new(),
            last_result: None,
        }
    }

    pub fn archive(&mut self, op: Box<dyn Operation>) {
        self.history.push(op)
    }
    pub fn last_op(&self) -> Option<&dyn Operation> {
        self.history.last().map(|x| &**x)
    }

    pub fn last_op_successful(&self, personality: &mut Personality, result: &Sandwich) -> bool {
        // First, apply the last operation to the last result.
        // Then, check if that matches the current result.
        if let Some(op) = self.last_op() {
            if let Some(last_res) = self.last_result.as_ref() {
                let imagined_result = op.apply(last_res.clone(), personality);
                // Only successful if there was some intended difference *and*
                // it was the correct difference.
                return last_res.ingredients != imagined_result.ingredients
                    && imagined_result.ingredients == result.ingredients;
            }
        }
        false
    }

    pub fn last_question_failed(&self, personality: &mut Personality, result: &Sandwich) -> bool {
        if let Some(op) = self.last_op() {
            if let Some(last_res) = self.last_result.as_ref() {
                let imagined_result = op.apply(last_res.clone(), personality);
                // Make sure it's a question (doesn't affect our sandwich).
                let is_question = last_res.ingredients == imagined_result.ingredients;
                // Negative result if our sandwich had to change because of the answer.
                return is_question && imagined_result.ingredients != result.ingredients;
            }
        }
        false
    }

    /// Based on the current conversation state and resulting sandwich, choose
    /// an operation to ask our conversation partner to apply to said sandwich.
    pub fn pick_op(
        &mut self,
        personality: &Personality,
        result: &Sandwich,
    ) -> Option<Box<dyn Operation>> {
        let mut rng = thread_rng();

        self.last_result = Some(result.clone());

        // If the result has all the ingredients we want, then we're finished.
        let has_all = self
            .desired
            .ingredients
            .iter()
            .all(|x| result.ingredients.contains(x));
        if has_all {
            return None;
        }

        // The basic behavior: pick the next ingredient on the sandwich.
        // Find the top-most shared ingredient between desired and result.
        let last_shared = self
            .desired
            .ingredients
            .iter()
            .enumerate()
            // Mostly filter out allergens.
            .filter(|(idx, x)| {
                !personality
                    .allergies
                    .iter()
                    .any(|a| &a.ingredient == *x && rng.gen_bool(a.severity))
            })
            .rfind(|(idx, x)| result.ingredients.contains(x));
        // We want to add the next one!
        let mut next_idx = last_shared.map(|(i, _)| i + 1).unwrap_or(0);
        println!("next index we want: {}", next_idx);

        // Always add the base bread first.
        if next_idx == 0 {
            return Some(Box::new(Add(
                self.desired.ingredients[next_idx].clone(),
                Relative::Top,
            )));
        }

        // Maybe forget this ingredient and move on to the next one.
        if rng.gen_bool(personality.forgetfulness) {
            next_idx += 1;
        }

        // TODO When considering a removal, maybe try to do a swap instead.

        // There could be extra ingredients that we didn't ask for.
        let extra = result
            .ingredients
            .iter()
            .find(|x| !self.desired.ingredients.contains(x));
        if let Some(extra) = extra {
            if !rng.gen_bool(personality.shyness / personality.stress()) {
                // FIXME Not *exactly* what we want, but close.
                let remover = Box::new(Remove(extra.clone()));
                return Some(if rng.gen_bool(personality.adverbs) {
                    Box::new(Persist(remover))
                } else {
                    remover
                });
            }
        }

        // There's a mistake if any preceding ingredients aren't in the result sandwich.
        // NOTE disregarding order for the moment.
        let mistake = self
            .desired
            .ingredients
            .iter()
            .take(next_idx)
            .position(|x| !result.ingredients.contains(x));
        // If we aren't shy, try to correct a mistake!
        if mistake.is_some()
            && !rng.gen_bool(personality.shyness / personality.stress())
            && rng.gen_bool((personality.adposition * 1.5).min(0.99))
        {
            let idx = mistake.unwrap();
            // Pick a preposition to position the missing ingredient where we'd like it.
            // TODO If this machine doesn't care about ordering, then just ask to add it to the end.
            // TODO if idx is zero or if this personality has a preference for Before.
            let rel = if idx == 0 {
                // Find first ingredient that comes after the missing one in our desires.
                let after = self
                    .desired
                    .ingredients
                    .iter()
                    .filter(|x| result.ingredients.contains(x))
                    .next();
                after.map(|b| Relative::Before(b.clone()))
            } else {
                // Find last ingredient in the result that comes before the missing one in our desires.
                let before = self
                    .desired
                    .ingredients
                    .iter()
                    .take(idx)
                    .filter(|x| result.ingredients.contains(x))
                    .last();
                // If we found one, place the missing ingredient after it.
                before.map(|b| Relative::After(b.clone()))
            }
            .unwrap_or(Relative::Top);
            return Some(Box::new(Add(self.desired.ingredients[idx].clone(), rel)));
        }

        // Check for allergens in the result sandwich.
        let allergen = personality
            .allergies
            .iter()
            // TODO Use contains logic here instead of exact match, allowing
            // allergies to whole categories.
            .filter(|a| result.ingredients.iter().any(|x| a.ingredient.includes(x)))
            .next();

        if let Some(allergen) = allergen {
            // If the allergy is severe and we aren't shy about it, ask for that
            // ingredient to be removed.
            if rng.gen_bool(allergen.severity)
                && !rng.gen_bool(personality.shyness / personality.stress())
                && rng.gen_bool((personality.adverbs * 1.5).min(0.99))
            {
                return Some(Box::new(Remove(allergen.ingredient.clone())));
            }
        }

        // TODO Change my mind about what I want based on my favorites.
        if rng.gen_bool((personality.spontaneity * personality.stress()).min(0.9)) {
            // If our previous desires contain too few of our favorites, then
            // add one in.
            let any_favs = self.desired.ingredients.iter().any(|x| {
                // Check if one of our favorites includes this ingredient.
                personality
                    .preferences
                    .iter()
                    .any(|fav| fav.ingredient.includes(x) && rng.gen_bool(fav.severity))
            });
            if !any_favs && !personality.preferences.is_empty() {
                // Pick a random favorite based on their severity.
                // NOTE Assumes every machine has at least one favorite.
                let weights = personality.preferences.iter().map(|x| x.severity);
                let dist =
                    WeightedIndex::new(weights).expect("Unable to make favorites distribution");
                let pick = dist.sample(&mut rng);
                return Some(Box::new(Add(
                    personality.preferences[pick].ingredient.clone(),
                    Relative::Top,
                )));
            }
        }

        let next_ingr = self.desired.ingredients.get(next_idx);
        next_ingr.map(|next_ingr| {
            // Maybe ask if they have the ingredient we want.
            if !self.desired.ensured.contains(next_ingr)
                && rng.gen_bool(personality.shyness / personality.stress())
            {
                return Box::new(CheckFor(next_ingr.clone())) as Box<dyn Operation>;
            }

            // If there are multiple of the ingredient we want, ask for them all at once.
            // Number of the ingredient we want in a row
            // TODO Check the whole list of remaining ingredients if order doesn't
            // matter to this machine.
            let adder = Box::new(Add(next_ingr.clone(), Relative::Top));
            let same_count = self
                .desired
                .ingredients
                .iter()
                .skip(next_idx) // If we want index 1, skip just the zeroth.
                .take_while(|x| x == &next_ingr)
                .count();
            if same_count > 1 && rng.gen_bool(personality.numbers) {
                Box::new(Repeat(same_count as u32, adder)) as Box<dyn Operation>
            } else {
                // Default behavior, just add the next ingredient to the top of the sandwich.
                adder as Box<dyn Operation>
            }
        })
    }
}

/// A single message of text and/or a sandwich.
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub text: Option<String>,
    pub sandwich: Option<Sandwich>,
}
impl Message {
    pub fn new(text: Option<String>, sandwich: Option<Sandwich>) -> Self {
        Self { text, sandwich }
    }
    /// Max size in bytes of a message.
    const MAX_SIZE: usize = 4096;
    pub async fn recv(stream: &mut TcpStream) -> anyhow::Result<Self> {
        let mut buf = [0u8; Self::MAX_SIZE];
        // Always read the same packet size.
        // TODO Just read until valid json is complete?
        stream.read_exact(&mut buf).await?;
        // Trim trailing zeroes for json parsing.
        let last_valid = buf.iter().rposition(|b| *b != 0).unwrap();
        Ok(serde_json::from_slice(&buf[0..=last_valid])?)
    }
    pub async fn send(&self, stream: &mut TcpStream) -> anyhow::Result<()> {
        let mut buf = [0u8; Self::MAX_SIZE];
        serde_json::to_writer(&mut buf as &mut [u8], self)?;
        stream.write(&buf).await?;
        Ok(())
    }
}
