mod allergy;
pub mod ops;
pub mod personality;

// Re-export everything from behavior submodules.
pub use allergy::*;
pub use ops::*;
pub use personality::*;

use crate::{
    grammar::{self, AnnotatedWord, PhraseNode, WordFunction, WordRole},
    sandwich::Sandwich,
};
use nom::{branch::*, combinator::*, sequence::*, IResult};
use rand;
use rand::prelude::*;

/// Basically a fixed iterator instance.
/// Having the ingredient index lets us develop spatial relationships to other ingredients.
/// TODO If we want to develop temporal relationships, we could store some history/memory here.
#[derive(Clone, Copy, Debug)]
pub struct PositionedIngredient<'a> {
    pub sandwich: &'a Sandwich,
    pub history: &'a [usize],
    pub index: usize,
}

pub type Behaviors = Vec<Box<dyn Behavior>>;

pub trait Behavior {
    fn start(&self);
    fn end(&self);
    fn next_ingredient(&mut self, sandwich: &Sandwich, pick: Option<usize>) -> Option<usize>;
}

const MAX_ACCURACY: f64 = 1.0;
const MIN_ACCURACY: f64 = 0.1;

#[derive(Clone, Default)]
pub struct Forgetful {
    /// How forgetful this machine is.
    degree: f64,
    forgotten: Vec<usize>,
}
impl Forgetful {
    pub fn new(degree: f64) -> Self {
        Self {
            degree,
            ..Default::default()
        }
    }
}
impl Behavior for Forgetful {
    fn start(&self) {}
    fn end(&self) {}
    fn next_ingredient(&mut self, sandwich: &Sandwich, pick: Option<usize>) -> Option<usize> {
        let mut rng = thread_rng();
        let curr_idx = pick.unwrap_or(0);
        // TODO Chance to remember a forgotten ingredient.
        if rng.gen_bool(self.degree * 0.5) && !self.forgotten.is_empty() {
            println!("remembering!!");
            Some(self.forgotten.remove(0))
        } else if pick.is_some()
            && sandwich.ingredients.len() > curr_idx
            && rng.gen_bool(self.degree)
        {
            println!("forgetting!");
            self.forgotten.push(curr_idx);
            if curr_idx + 1 < sandwich.ingredients.len() {
                Some(curr_idx + 1)
            } else {
                None
            }
        } else {
            pick
        }
    }
}

/// TODO Give this two traits! One for parsing, one for encoding!!
/// Each encoder may implement new parsing/encoding features for the language.
pub trait Encoder {
    fn encode(&mut self, lang: &Personality, item: PositionedIngredient) -> String;
    /// Given a phrase and a sandwich, produce the next step of the sandwich.
    /// Most basic version: find the object of the phrase, add that to the sandwich.
    fn decode(&mut self, phrase: &PhraseNode, sandwich: &mut Sandwich, lang: &Personality) -> bool;
    fn noun_phrase<'a>(
        &self,
        input: &'a [AnnotatedWord],
    ) -> IResult<&'a [AnnotatedWord], PhraseNode>;
}

/// A root encoder.
pub struct DesireEncoder;
// impl Encoder for DesireEncoder {
//     fn encode(&mut self, context: &Personality, item: PositionedIngredient) -> String {
//         let ingredient = &item.sandwich.ingredients[item.index];
//         let obj = context
//             .dictionary
//             .ingredients
//             .to_word(&ingredient, String::new())
//             .unwrap();
//         // TODO Support for negative here.
//         let (verb, _) = context.dictionary.word_for_def(WordFunction::Desire);
//         format!("{} {}", obj, verb)
//     }
//     fn noun_phrase<'a>(
//         &self,
//         input: &'a [AnnotatedWord],
//     ) -> IResult<&'a [AnnotatedWord], PhraseNode> {
//         map(grammar::noun, |n| PhraseNode::NounPhrase(vec![n]))(input)
//     }
//     fn decode(&mut self, phrase: &PhraseNode, sandwich: &mut Sandwich, lang: &Personality) -> bool {
//         match phrase.main_verb().and_then(|v| v.definition()) {
//             Some(WordFunction::Desire) => {
//                 let word = phrase.object();
//                 if let Some(WordFunction::Ingredient) =
//                     word.and_then(|o| o.entry.as_ref()).map(|e| e.function)
//                 {
//                     let ingredient = lang.dictionary.ingredients.from_word(&word.unwrap().word);
//                     sandwich.ingredients.push(ingredient.clone());
//                     return false;
//                 }
//             }
//             Some(WordFunction::Greeting) => {
//                 // Represents showing the sandwich to the client.
//                 println!("{:?}", sandwich);
//                 // End the conversation.
//                 return true;
//             }
//             _ => (),
//         }
//         false
//     }
// }

pub struct NegativeEncoder;
// impl Encoder for NegativeEncoder {
//     fn encode(&mut self, lang: &Personality, item: PositionedIngredient) -> String {
//         // TODO How do we know what we don't want?
//         todo!()
//     }
//     fn decode(&mut self, phrase: &PhraseNode, sandwich: &mut Sandwich, lang: &Personality) -> bool {
//         todo!()
//     }
//     fn noun_phrase<'a>(
//         &self,
//         input: &'a [AnnotatedWord],
//     ) -> IResult<&'a [AnnotatedWord], PhraseNode> {
//         todo!()
//     }
// }

pub enum HeadSide {
    Pre,
    Post,
}

pub struct RelativeEncoder {
    inner: Box<dyn Encoder>,
    side_bias: f64,
    accuracy: f64,
}
impl RelativeEncoder {
    pub fn new(accuracy: f64, inner: impl Encoder + 'static) -> Self {
        Self {
            inner: Box::new(inner),
            side_bias: 0.5,
            accuracy,
        }
    }
    fn pick_side(&mut self) -> HeadSide {
        let mut rng = thread_rng();
        let change = rng.gen_range(0.05, 0.15);
        if rng.gen_bool(self.side_bias) {
            if self.side_bias < 1.0 {
                self.side_bias += change;
            }
            HeadSide::Pre
        } else {
            if self.side_bias > 0.0 {
                self.side_bias -= change;
            }
            HeadSide::Post
        }
    }
}
// impl Encoder for RelativeEncoder {
//     fn encode(&mut self, context: &Personality, item: PositionedIngredient) -> String {
//         let last_order = *item.history.last().unwrap_or(&0);
//         println!("Encoding relative maybe? {:?}", item);
//         if item.index != last_order && item.index != last_order + 1 {
//             if item.index > 0 {
//                 // We want this ingredient after the closest one before it that we already
//                 // asked for.
//                 let previous = item
//                     .history
//                     .iter()
//                     .filter(|x| **x < item.index)
//                     .max()
//                     .map(|x| *x)
//                     .unwrap_or(item.history.len() - 1);
//                 let prev_word = context
//                     .dictionary
//                     .ingredients
//                     .to_word(&item.sandwich.ingredients[previous], String::new())
//                     .unwrap();
//                 let (prep, _) = context.dictionary.word_for_def(WordFunction::After);
//                 // Syntax is: V'[PP[NP P] V'...]
//                 match self.pick_side() {
//                     // TODO Allow the head/inner to switch sides!
//                     HeadSide::Pre => format!(
//                         "{} {} {}",
//                         self.inner.encode(context, item),
//                         prep,
//                         prev_word,
//                     ),
//                     HeadSide::Post => format!(
//                         "{} {} {}",
//                         prev_word,
//                         prep,
//                         self.inner.encode(context, item),
//                     ),
//                 }
//             } else {
//                 // We want this ingredient after the closest one before it that we already
//                 // asked for.
//                 let next = item
//                     .history
//                     .iter()
//                     .filter(|x| **x > item.index)
//                     .min()
//                     .map(|x| *x)
//                     .unwrap_or(0);
//                 let prev_word = context
//                     .dictionary
//                     .ingredients
//                     .to_word(&item.sandwich.ingredients[next], String::new())
//                     .unwrap();
//                 let (prep, _) = context.dictionary.word_for_def(WordFunction::Before);
//                 // Syntax is: V'[PP[NP P] V'...]
//                 match self.pick_side() {
//                     // TODO Allow the head/inner to switch sides!
//                     HeadSide::Pre => format!(
//                         "{} {} {}",
//                         self.inner.encode(context, item),
//                         prep,
//                         prev_word,
//                     ),
//                     HeadSide::Post => format!(
//                         "{} {} {}",
//                         prev_word,
//                         prep,
//                         self.inner.encode(context, item),
//                     ),
//                 }
//             }
//         } else {
//             self.inner.encode(context, item)
//         }
//     }
//     // fn noun_phrase<'a>(
//     //     &self,
//     //     input: &'a [AnnotatedWord],
//     // ) -> IResult<&'a [AnnotatedWord], PhraseNode> {
//     //     alt((
//     //         map(
//     //             tuple((
//     //                 |i| self.inner.noun_phrase(i),
//     //                 preposition,
//     //                 |i| self.inner.noun_phrase(i),
//     //             )),
//     //             |(np1, p, np2)| PhraseNode::PositionalPhrase(vec![np1, p, np2]),
//     //         ),
//     //         |i| self.inner.noun_phrase(i),
//     //     ))(input)
//     // }
//     // TODO Make a DecodeError or DecodeResult type to represent either Completed,
//     // Success, or Failure.
//     // fn decode(&mut self, phrase: &PhraseNode, sandwich: &mut Sandwich, lang: &Personality) -> bool {
//     //     // Even if we successfully parse a positional phrase,
//     //     // there's a chance we miss it.
//     //     if thread_rng().gen_bool(self.accuracy) {
//     //         // look for a prepositional phrase in the object of the main verb.
//     //         if let Some(PhraseNode::PositionalPhrase(parts)) = phrase.object_phrase() {
//     //             if let [np1, p, np2] = &parts[..] {
//     //                 let p = p.pos().unwrap();
//     //                 // TODO Use successfulness to change bias rather than just by random.
//     //                 let (existing_np, new_np) = match self.pick_side() {
//     //                     HeadSide::Pre => (np2, np1),
//     //                     HeadSide::Post => (np1, np2),
//     //                 };
//     //                 // The ingredient presumably already in the sandwich
//     //                 let existing = lang
//     //                     .dictionary
//     //                     .ingredients
//     //                     .from_word(&existing_np.object().unwrap().word);
//     //                 let new = lang
//     //                     .dictionary
//     //                     .ingredients
//     //                     .from_word(&new_np.object().unwrap().word);
//     //                 // Index of the existing ingredient.
//     //                 let idx = sandwich.ingredients.iter().position(|x| x == existing);
//     //                 if let Some(idx) = idx {
//     //                     // TODO Consider which type of position it is. For now assuming "after".
//     //                     let dest_idx = match p.definition() {
//     //                         Some(WordFunction::After) => idx + 1,
//     //                         Some(WordFunction::Before) => idx,
//     //                         _ => idx,
//     //                     };
//     //                     sandwich.ingredients.insert(dest_idx, new.clone());
//     //                     // We successfully used a positional phrase, so up our accuracy!
//     //                     if self.accuracy < MAX_ACCURACY {
//     //                         self.accuracy += 0.1;
//     //                     }
//     //                     return false;
//     //                 }
//     //             }
//     //         } else
//     //         // If we never see positions, slowly forget about them.
//     //         if self.accuracy > MIN_ACCURACY {
//     //             self.accuracy -= 0.001;
//     //         }
//     //     }
//     //     // If we didn't find the referred to ingredient, just add this one to
//     //     // the end.
//     //     self.inner.decode(phrase, sandwich, lang)
//     // }
// }

// fn preposition(input: &[AnnotatedWord]) -> IResult<&[AnnotatedWord], PhraseNode> {
//     if input.len() > 0 && input[0].role == Some(WordRole::Preposition) {
//         let rest = &input[1..];
//         Ok((rest, PhraseNode::Position(input[0].clone())))
//     } else {
//         Err(nom::Err::Error((input, nom::error::ErrorKind::IsA)))
//     }
// }
