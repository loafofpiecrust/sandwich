use crate::{behavior::Encoder, client::Language, sandwich::Ingredient};
use itertools::Itertools;
use nom::{branch::*, bytes::complete::*, combinator::*, multi::*, sequence::*, IResult, *};
use serde::Deserialize;
use serde_yaml;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    fs::File,
};

pub struct Dictionary {
    words: HashMap<String, DictionaryEntry>,
    pub ingredients: Ingredient,
}
impl Dictionary {
    pub fn new() -> Self {
        let file = File::open("dictionary.yml").unwrap();
        let mut words: HashMap<String, DictionaryEntry> = serde_yaml::from_reader(file).unwrap();
        let ingredients = Ingredient::all();
        words.extend(ingredients.leaves().into_iter().map(|(n, x)| {
            (
                x,
                DictionaryEntry {
                    function: WordFunction::Ingredient,
                    role: WordRole::Noun,
                    definition: n,
                },
            )
        }));

        Self { words, ingredients }
    }
    pub fn first_word_in_class(&self, category: WordFunction) -> (&str, &DictionaryEntry) {
        for (word, entry) in &self.words {
            if entry.function == category {
                return (word, entry);
            }
        }
        unreachable!("There should be at least one word per function.")
    }
    pub fn get(&self, word: &str) -> Option<&DictionaryEntry> {
        self.words.get(word)
    }
}

pub struct Context {}
impl Default for Context {
    fn default() -> Self {
        Self {}
    }
}
impl Context {}

#[derive(PartialEq, Debug, Copy, Clone, Deserialize)]
pub enum WordFunction {
    Me,
    You,
    Greeting,
    Affirmation,
    Negation,
    Pronoun,
    Action,
    Desire,
    After,
    // Lexical Functions
    Sandwich,
    /// Has some meaning beyond function.
    Ingredient,
}

/// Analogous to part of speech.
#[derive(PartialEq, Debug, Copy, Clone, Deserialize)]
pub enum WordRole {
    // Functional roles
    /// Things like greetings, affirmations, etc.
    Special,

    // Lexical roles
    /// to want, to prepend, to append, to stick in middle, etc.
    Verb,
    /// Ingredients and pronouns
    Noun,
    /// *Between* x and y
    Preposition,
    /// x *and* y
    NounConjunction,
}

#[derive(Deserialize, Clone, Debug)]
pub struct DictionaryEntry {
    pub function: WordFunction,
    pub role: WordRole,
    pub definition: String,
}

/// Syllables are always two characters, CV.
#[derive(Debug, Clone)]
pub struct Syllable(char, char);
impl Display for Syllable {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.0, self.1)
    }
}

#[derive(Debug, Clone)]
pub struct Word(pub Vec<Syllable>);
impl Display for Word {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for syllable in &self.0 {
            write!(f, "{}", syllable)?;
        }
        Ok(())
    }
}
pub type Phrase = Vec<Word>;
pub type AnnotatedPhrase = Vec<AnnotatedWord>;

#[derive(Debug, Clone)]
pub struct AnnotatedWord {
    // TODO: Syllables -> Morphemes
    pub word: Word,
    /// Part of speech based on syntactic context
    pub role: Option<WordRole>,
    pub entry: Option<DictionaryEntry>,
}
impl AnnotatedWord {
    pub fn definition(&self) -> Option<&WordFunction> {
        self.entry.as_ref().map(|e| &e.function)
    }
}

pub static CONSONANTS: &str = "ptkhmnwls";
pub static VOWELS: &str = "aioue";

fn consonant(input: &[u8]) -> IResult<&[u8], char> {
    character::complete::one_of(CONSONANTS)(input)
}

fn vowel(input: &[u8]) -> IResult<&[u8], char> {
    character::complete::one_of(VOWELS)(input)
}

pub fn syllable_cv(input: &[u8]) -> IResult<&[u8], Syllable> {
    let (input, c) = consonant(input)?;
    let (input, v) = vowel(input)?;
    Ok((input, Syllable(c, v)))
}
pub fn syllable_vc(input: &[u8]) -> IResult<&[u8], Syllable> {
    let (input, c) = vowel(input)?;
    let (input, v) = consonant(input)?;
    Ok((input, Syllable(c, v)))
}
pub fn syllable(input: &[u8]) -> IResult<&[u8], Syllable> {
    alt((syllable_cv, syllable_vc))(input)
}

pub fn word(input: &[u8]) -> IResult<&[u8], Word> {
    let (input, syllables) = many1(syllable)(input)?;
    Ok((input, Word(syllables)))
}

pub fn phrase(input: &[u8]) -> IResult<&[u8], Phrase> {
    terminated(separated_list(tag(" "), word), opt(tag("\n")))(input)
}

pub fn annotate(phrase: Phrase, context: &Language) -> AnnotatedPhrase {
    let mut result = AnnotatedPhrase::new();
    for word in phrase {
        let word_str = word.to_string();
        let entry = context.dictionary.get(&word_str);
        result.push(AnnotatedWord {
            word,
            // TODO: Use syntactic context for word role.
            role: entry.map(|e| e.role.clone()),
            entry: entry.cloned(),
        });
    }
    result
}

pub fn sentence(input: &[u8], lang: &Language, encoder: &dyn Encoder) -> Option<PhraseNode> {
    if let Ok((_, parsed)) = phrase(input) {
        let tagged = annotate(parsed, lang);
        if let Ok((_, tree)) = clause(&tagged, encoder) {
            std::dbg!(&tree);
            Some(tree)
        } else {
            // Try again with unknown words removed.
            let nt = tagged
                .into_iter()
                .filter(|x| x.role.is_some() || x.entry.is_some())
                .collect_vec();
            let c = clause(&nt, encoder);
            c.ok().map(|(_, t)| t)
        }
    } else {
        None
    }
}

#[derive(Debug)]
pub enum PhraseNode {
    NounPhrase(Vec<PhraseNode>),
    VerbPhrase(Vec<PhraseNode>),
    ClausalPhrase(Vec<PhraseNode>),
    Noun(AnnotatedWord),
    Verb(AnnotatedWord),
    Position(AnnotatedWord),
    PositionalPhrase(Vec<PhraseNode>),
}
impl PhraseNode {
    // TODO Handle special phrases like greetings.
    pub fn main_verb(&self) -> Option<&AnnotatedWord> {
        use PhraseNode::*;
        match self {
            Verb(x) => Some(x),
            NounPhrase(x) | VerbPhrase(x) | ClausalPhrase(x) | PositionalPhrase(x) => {
                x.iter().filter_map(|x| x.main_verb()).next()
            }
            _ => None,
        }
    }
    pub fn main_verb_phrase(&self) -> Option<&PhraseNode> {
        use PhraseNode::*;
        match self {
            VerbPhrase(_) => Some(self),
            NounPhrase(x) | ClausalPhrase(x) | PositionalPhrase(x) => {
                x.iter().filter_map(|x| x.main_verb_phrase()).next()
            }
            _ => None,
        }
    }
    pub fn object_phrase(&self) -> Option<&PhraseNode> {
        use PhraseNode::*;
        match self {
            NounPhrase(_) | PositionalPhrase(_) => Some(self),
            VerbPhrase(x) => x.iter().filter_map(|x| x.object_phrase()).next(),
            ClausalPhrase(x) => x
                .iter()
                .filter_map(|x| {
                    // Objects only come from the verb phrase, subjects sit outside.
                    if let VerbPhrase(_) = x {
                        x.object_phrase()
                    } else {
                        None
                    }
                })
                .next(),
            _ => None,
        }
    }
    pub fn object(&self) -> Option<&AnnotatedWord> {
        use PhraseNode::*;
        match self {
            Noun(x) => Some(x),
            VerbPhrase(x) | NounPhrase(x) | PositionalPhrase(x) => {
                x.iter().filter_map(|x| x.object()).next()
            }
            ClausalPhrase(x) => x
                .iter()
                .filter_map(|x| {
                    // Objects only come from the verb phrase, subjects sit outside.
                    if let VerbPhrase(_) = x {
                        x.object()
                    } else {
                        None
                    }
                })
                .next(),
            _ => None,
        }
    }
    pub fn subject(&self) -> Option<&AnnotatedWord> {
        use PhraseNode::*;
        match self {
            Noun(x) => Some(x),
            NounPhrase(x) => x.iter().filter_map(|x| x.subject()).next(),
            ClausalPhrase(x) => x
                .iter()
                .filter_map(|x| {
                    if let NounPhrase(_) = x {
                        x.subject()
                    } else {
                        None
                    }
                })
                .next(),
            _ => None,
        }
    }
    pub fn subtitles(&self) -> String {
        use PhraseNode::*;
        match self {
            ClausalPhrase(x) | NounPhrase(x) | VerbPhrase(x) | PositionalPhrase(x) => {
                x.iter().map(|x| x.subtitles()).join(" ")
            }
            Noun(x) | Verb(x) | Position(x) => x.entry.as_ref().unwrap().definition.clone(),
        }
    }
}

pub fn noun(input: &[AnnotatedWord]) -> IResult<&[AnnotatedWord], PhraseNode> {
    if input.len() > 0 && input[0].role == Some(WordRole::Noun) {
        let rest = &input[1..];
        Ok((rest, PhraseNode::Noun(input[0].clone())))
    } else {
        Err(nom::Err::Error((input, nom::error::ErrorKind::IsA)))
    }
}

pub fn verb(input: &[AnnotatedWord]) -> IResult<&[AnnotatedWord], PhraseNode> {
    if input.len() > 0 && input[0].role == Some(WordRole::Verb) {
        let rest = &input[1..];
        Ok((rest, PhraseNode::Verb(input[0].clone())))
    } else {
        Err(nom::Err::Error((input, nom::error::ErrorKind::IsA)))
    }
}

/// NP -> N
// pub fn noun_phrase(input: &[AnnotatedWord]) -> IResult<&[AnnotatedWord], PhraseNode> {
//     map(noun, |n| PhraseNode::NounPhrase(vec![n]))(input)
// }

/// VP -> (NP) V
pub fn verb_phrase<'a>(
    input: &'a [AnnotatedWord],
    encoder: &dyn Encoder,
) -> IResult<&'a [AnnotatedWord], PhraseNode> {
    map(pair(opt(|i| encoder.noun_phrase(i)), verb), |(np, v)| {
        let mut parts = Vec::new();
        if let Some(np) = np {
            parts.push(np);
        }
        parts.push(v);
        PhraseNode::VerbPhrase(parts)
    })(input)
}

/// CP -> NP VP
pub fn clause<'a>(
    input: &'a [AnnotatedWord],
    encoder: &dyn Encoder,
) -> IResult<&'a [AnnotatedWord], PhraseNode> {
    alt((
        // We use different branches here instead of an optional subject because nom
        // consumes from the left, and we don't want to misidentify an OV sentence as SV.
        map(
            |i| verb_phrase(i, encoder),
            |vp| PhraseNode::ClausalPhrase(vec![vp]),
        ),
        map(
            pair(|i| encoder.noun_phrase(i), |i| verb_phrase(i, encoder)),
            |(np, vp)| PhraseNode::ClausalPhrase(vec![np, vp]),
        ),
    ))(input)
}
