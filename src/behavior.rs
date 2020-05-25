use crate::grammar::{self, AnnotatedWord, Context, PhraseNode, WordFunction, WordRole};
use crate::sandwich::{Ingredient, Sandwich};
use nom::{
    branch::*, bytes::complete::*, character::complete::*, combinator::*, multi::*, named, one_of,
    sequence::*, take_while, ws, IResult, *,
};
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

pub enum OrderStatus {
    Ingredient(usize),
    None,
    Finished,
}

pub trait Behavior {
    fn start(&self);
    fn end(&self);
    fn next_ingredient(&mut self, sandwich: &Sandwich, pick: Option<usize>) -> Option<usize>;
}

#[derive(Clone, Default)]
pub struct Forgetful {
    forgotten: Vec<usize>,
}
impl Behavior for Forgetful {
    fn start(&self) {}
    fn end(&self) {}
    fn next_ingredient(&mut self, sandwich: &Sandwich, pick: Option<usize>) -> Option<usize> {
        let mut rng = thread_rng();
        let curr_idx = pick.unwrap_or(0);
        // TODO Chance to remember a forgotten ingredient.
        if rng.gen_bool(0.1) && !self.forgotten.is_empty() {
            println!("remembering!!");
            Some(self.forgotten.remove(0))
        } else if pick.is_some() && sandwich.ingredients.len() > curr_idx && rng.gen_bool(0.1) {
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

// pub struct InOrder;
// impl Behavior for InOrder {
//     fn start(&self) {}
//     fn end(&self) {}
//     fn next_ingredient(&mut self, sandwich: &Sandwich, pick: Option<usize>) -> Option<usize> {
//         Some(pick.unwrap_or(0))
//     }
// }

enum Resolvable {
    Resolved(String),
    Unresolved(usize),
}
impl Resolvable {
    fn to_string(self) -> Option<String> {
        if let Resolvable::Resolved(s) = self {
            Some(s)
        } else {
            None
        }
    }
}

/// TODO Give this two traits! One for parsing, one for encoding!!
/// Each encoder may implement new parsing/encoding features for the language.
pub trait Encoder {
    fn encode(&mut self, context: &Context, item: PositionedIngredient) -> String;
    fn noun_phrase<'a>(
        &self,
        input: &'a [AnnotatedWord],
    ) -> IResult<&'a [AnnotatedWord], PhraseNode>;
}

/// A root encoder.
pub struct DesireEncoder;
impl Encoder for DesireEncoder {
    fn encode(&mut self, context: &Context, item: PositionedIngredient) -> String {
        let ingredient = &item.sandwich.ingredients[item.index];
        let obj = context
            .dictionary
            .ingredients
            .to_word(&ingredient, String::new())
            .unwrap();
        let (verb, _) = context.dictionary.first_word_in_class(WordFunction::Desire);
        format!("{} {}", obj, verb)
    }
    fn noun_phrase<'a>(
        &self,
        input: &'a [AnnotatedWord],
    ) -> IResult<&'a [AnnotatedWord], PhraseNode> {
        map(grammar::noun, |n| PhraseNode::NounPhrase(vec![n]))(input)
    }
}

pub enum HeadSide {
    Pre,
    Post,
}

pub struct RelativeEncoder {
    inner: Box<dyn Encoder>,
    side: HeadSide,
}
impl RelativeEncoder {
    pub fn new(inner: impl Encoder + 'static) -> Self {
        Self {
            inner: Box::new(inner),
            side: HeadSide::Post,
        }
    }
}
impl Encoder for RelativeEncoder {
    fn encode(&mut self, context: &Context, item: PositionedIngredient) -> String {
        let last_order = *item.history.last().unwrap_or(&0);
        println!("Encoding relative maybe? {:?}", item);
        if item.index != last_order && item.index != last_order + 1 && item.index > 0 {
            let previous = &item.sandwich.ingredients[item.index - 1];
            let prev_word = context
                .dictionary
                .ingredients
                .to_word(&previous, String::new())
                .unwrap();
            let (prep, _) = context.dictionary.first_word_in_class(WordFunction::After);
            // Syntax is: V'[PP[NP P] V'...]
            match &self.side {
                HeadSide::Pre => format!(
                    "{} {} {}",
                    prep,
                    prev_word,
                    self.inner.encode(context, item)
                ),
                HeadSide::Post => format!(
                    "{} {} {}",
                    prev_word,
                    prep,
                    self.inner.encode(context, item)
                ),
            }
        } else {
            self.inner.encode(context, item)
        }
    }
    fn noun_phrase<'a>(
        &self,
        input: &'a [AnnotatedWord],
    ) -> IResult<&'a [AnnotatedWord], PhraseNode> {
        let pp = |input| match &self.side {
            HeadSide::Pre => map(
                pair(preposition, |input| self.inner.noun_phrase(input)),
                |(p, np)| PhraseNode::PositionalPhrase(vec![p, np]),
            )(input),
            HeadSide::Post => map(
                pair(|input| self.inner.noun_phrase(input), preposition),
                |(np, p)| PhraseNode::PositionalPhrase(vec![np, p]),
            )(input),
        };
        alt((pp, |input| self.inner.noun_phrase(input)))(input)
    }
}

fn preposition(input: &[AnnotatedWord]) -> IResult<&[AnnotatedWord], PhraseNode> {
    if input.len() > 0 && input[0].role == Some(WordRole::Preposition) {
        let rest = &input[1..];
        Ok((rest, PhraseNode::Position(input[0].clone())))
    } else {
        Err(nom::Err::Error((input, nom::error::ErrorKind::IsA)))
    }
}
