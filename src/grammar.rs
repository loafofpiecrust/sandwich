use crate::behavior::{ops, Operation};
use crate::{behavior::personality::Personality, sandwich::Ingredient};
use itertools::Itertools;
use nom::{branch::*, bytes::complete::*, combinator::*, multi::*, sequence::*, IResult, *};
use rand::prelude::*;
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
    pub fn word_for_def(&self, category: WordFunction) -> (&str, &DictionaryEntry) {
        for (word, entry) in &self.words {
            if entry.function == category {
                return (word, entry);
            }
        }
        unreachable!("There should be at least one word per function.")
    }
    pub fn word_for_num(&self, number: u32) -> (&str, &DictionaryEntry) {
        let num_str = number.to_string();
        for (word, entry) in &self.words {
            if entry.function == WordFunction::Number && entry.definition == num_str {
                return (word, entry);
            }
        }
        todo!("There is no word for number {}", number)
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
    Have,
    After,
    Before,
    And,
    Ever,
    /// Please and thank you.
    Polite,
    // Lexical Functions
    Sandwich,
    Number,
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
    Adjective,
    Adverb,
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

pub fn annotate(phrase: Phrase, context: &Personality) -> AnnotatedPhrase {
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

pub fn sentence_new(input: &[u8], lang: &Personality) -> Option<Box<dyn Operation>> {
    phrase(input).ok().and_then(|(_, parsed)| {
        let tagged = annotate(parsed, lang);
        if let Ok((_, op)) = sentence(&tagged, lang) {
            Some(op)
        } else {
            // Try again with unknown words removed.
            let nt = tagged
                .into_iter()
                .filter(|x| x.role.is_some() || x.entry.is_some())
                .collect_vec();
            let c = sentence(&nt, lang);
            c.ok().map(|(_, t)| t)
        }
    })
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
    Empty,
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
    pub fn pos(&self) -> Option<&AnnotatedWord> {
        use PhraseNode::*;
        match self {
            Position(x) => Some(x),
            PositionalPhrase(x) | NounPhrase(x) | VerbPhrase(x) | ClausalPhrase(x) => {
                x.iter().filter_map(|x| x.pos()).next()
            }
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
            Empty => String::new(),
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

fn ingredient<'a>(
    input: &'a [AnnotatedWord],
    lang: &Personality,
) -> IResult<&'a [AnnotatedWord], Ingredient> {
    map(
        |i| word_with_def(i, WordFunction::Ingredient),
        |w| lang.dictionary.ingredients.from_word(&w.word).clone(),
    )(input)
}

fn word_with_def(
    input: &[AnnotatedWord],
    def: WordFunction,
) -> IResult<&[AnnotatedWord], &AnnotatedWord> {
    if let Some(d) = input
        .get(0)
        .and_then(|i| i.entry.as_ref())
        .map(|e| e.function)
    {
        if d == def {
            return Ok((&input[1..], &input[0]));
        }
    }
    Err(nom::Err::Error((input, nom::error::ErrorKind::IsA)))
}

pub fn word_with_role(
    input: &[AnnotatedWord],
    role: WordRole,
) -> IResult<&[AnnotatedWord], &AnnotatedWord> {
    if input.len() > 0 && input[0].role == Some(role) {
        let rest = &input[1..];
        Ok((rest, &input[0]))
    } else {
        Err(nom::Err::Error((input, nom::error::ErrorKind::IsA)))
    }
}

/// Matches a negated phrase to reverse the inner meaning, either "not A" or just "A".
fn adv_p<'a>(
    input: &'a [AnnotatedWord],
    lang: &Personality,
) -> IResult<&'a [AnnotatedWord], Box<dyn Operation>> {
    if thread_rng().gen_bool(lang.negation) {
        alt((
            map(
                pair(|i| word_with_role(i, WordRole::Adverb), |i| adv_p(i, lang)),
                |(adv, vp)| match adv.definition() {
                    Some(WordFunction::Ever) => Box::new(ops::Persist(vp)) as Box<dyn Operation>,
                    Some(WordFunction::Negation) => vp.reverse(),
                    _ => todo!(),
                },
            ),
            |i| pos_p(i, lang),
        ))(input)
    } else {
        pos_p(input, lang)
    }
}

fn adposition<'a>(
    input: &'a [AnnotatedWord],
    lang: &Personality,
) -> IResult<&'a [AnnotatedWord], ops::Relative> {
    map(
        pair(
            |i| ingredient(i, lang),
            |i| word_with_role(i, WordRole::Preposition),
        ),
        |(ingr, pos)| ops::Relative::from_def(pos.entry.as_ref().unwrap().function, ingr),
    )(input)
}

fn number(input: &[AnnotatedWord]) -> IResult<&[AnnotatedWord], u32> {
    map_res(
        |i| word_with_def(i, WordFunction::Number),
        |w| w.entry.as_ref().unwrap().definition.parse::<u32>(),
    )(input)
}

/// Matches numbered phrases, either "do A, X times" or just "A".
fn numbered_p<'a>(
    input: &'a [AnnotatedWord],
    lang: &Personality,
) -> IResult<&'a [AnnotatedWord], Box<dyn Operation>> {
    if thread_rng().gen_bool(lang.numbers) {
        alt((
            map(pair(number, |i| numbered_p(i, lang)), |(n, vp)| {
                Box::new(ops::Repeat(n, vp)) as Box<dyn Operation>
            }),
            |i| adv_p(i, lang),
        ))(input)
    } else {
        adv_p(input, lang)
    }
}

/// Matches prepositional phrases, either "A prep B" or just "A".
fn pos_p<'a>(
    input: &'a [AnnotatedWord],
    lang: &Personality,
) -> IResult<&'a [AnnotatedWord], Box<dyn Operation>> {
    if thread_rng().gen_bool(lang.adposition) {
        alt((
            |i| adposition(i, lang).and_then(|(i, r)| clause_new(i, &r, lang)),
            |i| clause_new(i, &ops::Relative::Top, lang),
        ))(input)
    } else {
        clause_new(input, &ops::Relative::Top, lang)
    }
}

fn greeting<'a>(input: &'a [AnnotatedWord]) -> IResult<&'a [AnnotatedWord], Box<dyn Operation>> {
    map(
        |i| word_with_def(i, WordFunction::Greeting),
        |_| Box::new(ops::Finish) as Box<dyn Operation>,
    )(input)
}

/// Top level sentence parser, either some general phrase or a special one like
/// a greeting.
fn sentence<'a>(
    input: &'a [AnnotatedWord],
    lang: &Personality,
) -> IResult<&'a [AnnotatedWord], Box<dyn Operation>> {
    alt((|i| conjuncted_phrase(i, lang), greeting))(input)
}

/// VP -> (NP) V
// TODO Add probability to understand preposition.
pub fn clause_new<'a>(
    input: &'a [AnnotatedWord],
    pos: &ops::Relative,
    lang: &Personality,
) -> IResult<&'a [AnnotatedWord], Box<dyn Operation>> {
    map(
        pair(
            |i| ingredient(i, lang),
            |i| word_with_role(i, WordRole::Verb),
        ),
        |(np, v)| match v.definition() {
            Some(WordFunction::Desire) => Box::new(ops::Add(np, pos.clone())) as Box<dyn Operation>,
            Some(WordFunction::Have) => Box::new(ops::Ensure(np)) as Box<dyn Operation>,
            _ => todo!("This verb hasn't been mapped to an operation yet."),
        },
    )(input)
}

/// Matches "A and B" or just "A"
/// TODO Move around the position of the conjunction.
fn conjuncted_phrase<'a>(
    input: &'a [AnnotatedWord],
    lang: &Personality,
) -> IResult<&'a [AnnotatedWord], Box<dyn Operation>> {
    let inner = |i| numbered_p(i, lang);
    if thread_rng().gen_bool(lang.conjunction) {
        alt((
            map(
                separated_pair(
                    inner,
                    |i| word_with_def(i, WordFunction::And),
                    // Allow recursion on conjunctions for X and (X and X), etc.
                    |i| conjuncted_phrase(i, lang),
                ),
                |(a, b)| Box::new(ops::Compound(a, b)) as Box<dyn Operation>,
            ),
            inner,
        ))(input)
    } else {
        inner(input)
    }
}
