use crate::sandwich::{Ingredient, Sandwich};
use crate::state::{Idle, State};
use nom::{
    branch::*, bytes::complete::*, character::complete::*, combinator::*, multi::*, named, one_of,
    sequence::*, take_while, ws, IResult, *,
};
use serde::{Deserialize, Serialize};
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
        let content = serde_yaml::from_reader(file).unwrap();
        Self {
            words: content,
            ingredients: Ingredient::all(),
        }
    }
    pub fn first_word_in_class(&self, category: WordFunction) -> &str {
        for (word, entry) in &self.words {
            if entry.function == category {
                return &word;
            }
        }
        unreachable!("There should be at least one word per function.")
    }
    pub fn get(&self, word: &str) -> Option<&DictionaryEntry> {
        self.words.get(word)
    }
}

pub struct Context {
    /// We'll have a few words with default parts of speech if totally ambiguous.
    pub dictionary: Dictionary,
    pub state: Box<dyn State>,
}
impl Default for Context {
    fn default() -> Self {
        Self {
            dictionary: Dictionary::new(),
            state: Box::new(Idle),
        }
    }
}
impl Context {
    pub fn respond(&mut self, input: &AnnotatedPhrase) -> (String, Option<Sandwich>) {
        let (response, sandwich, next_state) = self.state.respond(input, &self.dictionary);
        if let Some(next) = next_state {
            self.state = next;
        }
        (response, sandwich)
    }
}

#[derive(PartialEq, Debug, Copy, Clone, Deserialize)]
pub enum WordFunction {
    Greeting,
    Affirmation,
    Negation,
    Pronoun,
    Action,
    Desire,
    After,
    // Lexical Functions
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

pub static CONSONANTS: &str = "ptkhmnwl";
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

pub fn annotate(phrase: &Phrase, context: &Context) -> AnnotatedPhrase {
    let mut result = AnnotatedPhrase::new();
    for word in phrase {
        let word_str = format!("{}", word);
        let entry = context.dictionary.get(&word_str).map(|x| x.clone());
        result.push(AnnotatedWord {
            word: word.clone(),
            // TODO: Use syntactic context for word role.
            role: entry.clone().map(|e| e.clone().role),
            entry,
        });
    }
    result
}

pub fn sentence(input: &[u8], context: &Context) -> Option<PhraseNode> {
    if let Ok((_, parsed)) = phrase(input) {
        let tagged = annotate(&parsed, context);
        if let Ok((_, tree)) = clause(&tagged) {
            Some(tree)
        } else {
            None
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
pub fn noun_phrase(input: &[AnnotatedWord]) -> IResult<&[AnnotatedWord], PhraseNode> {
    map(noun, |n| PhraseNode::NounPhrase(vec![n]))(input)
}

/// VP -> (NP) V
pub fn verb_phrase(input: &[AnnotatedWord]) -> IResult<&[AnnotatedWord], PhraseNode> {
    map(pair(opt(noun_phrase), verb), |(np, v)| {
        let mut parts = Vec::new();
        if let Some(np) = np {
            parts.push(np);
        }
        parts.push(v);
        PhraseNode::VerbPhrase(parts)
    })(input)
}

/// CP -> NP VP
pub fn clause(input: &[AnnotatedWord]) -> IResult<&[AnnotatedWord], PhraseNode> {
    map(pair(noun_phrase, verb_phrase), |(np, vp)| {
        PhraseNode::ClausalPhrase(vec![np, vp])
    })(input)
}

struct Parsers {}
