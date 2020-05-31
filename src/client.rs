use crate::{
    audio,
    behavior::{Behavior, DesireEncoder, Encoder, PositionedIngredient, RelativeEncoder},
    comm,
    display::{setup_display, Render, RenderSender},
    grammar,
    grammar::WordFunction,
    sandwich::Sandwich,
    state::{Idle, State},
    wait_randomly,
};
use async_std::net::TcpStream;
use async_std::prelude::*;
use bincode;
use futures::{pin_mut, select, FutureExt};
use grammar::{sentence, Dictionary, PhraseNode};
use seqalign::{measures::LevenshteinDamerau, Align};
use std::{thread, time::Duration};

pub struct Language {
    pub dictionary: Dictionary,
    pub display: RenderSender,
}
impl Language {
    fn new() -> Self {
        Self {
            dictionary: Dictionary::new(),
            display: setup_display(),
        }
    }
    pub fn render(&self, state: Render) -> anyhow::Result<()> {
        Ok(self.display.send(state)?)
    }
}

pub struct Client {
    /// We'll have a few words with default parts of speech if totally ambiguous.
    pub state: Box<dyn State>,
    behaviors: Vec<Box<dyn Behavior>>,
    pub sandwich: Option<Sandwich>,
    pub history: Vec<usize>,
    next_index: usize,
    pub lang: Language,
    encoder: Box<dyn Encoder>,
}
impl Client {
    pub fn new() -> Self {
        Self {
            state: Box::new(Idle),
            behaviors: Vec::new(),
            history: Vec::new(),
            sandwich: None,
            next_index: 0,
            lang: Language::new(),
            encoder: Box::new(RelativeEncoder::new(0.8, DesireEncoder)),
        }
    }

    pub async fn connect_with_peer(&mut self) -> anyhow::Result<()> {
        let client = comm::find_peer().fuse();
        let server = comm::wait_for_peer().fuse();
        pin_mut!(client, server);
        select! {
            s = client => self.client(s?).await,
            s = server => self.server(s?).await,
        }
    }

    async fn server(&mut self, mut stream: TcpStream) -> anyhow::Result<()> {
        loop {
            // Wait for a request,
            let mut buf = [0; 512];
            stream.read(&mut buf).await?;
            let request: String = dbg!(bincode::deserialize(&buf)?);

            // Then respond with words and maybe a sandwich.
            let (resp, sandwich) = self.respond(&request);
            println!("Responding with {}", resp);
            self.say_phrase(&resp, sandwich, &mut stream).await?;
        }
    }

    async fn client(&mut self, mut server: TcpStream) -> anyhow::Result<()> {
        // Initial greeting phase!
        self.start_order(&mut server).await?;

        dbg!(&self.sandwich);

        // List all the ingredients I want.
        while let Some(line) = self.next_phrase() {
            self.say_phrase(&line, None, &mut server).await?;

            // Wait for a response.
            let response: String = {
                let mut buffer = [0; 512];
                server.read(&mut buffer).await?;
                bincode::deserialize(&buffer)?
            };
            let sandwich: Option<Sandwich> = {
                let mut buffer = [0; 512];
                server.read(&mut buffer).await?;
                bincode::deserialize(&buffer)?
            };

            println!("{}", response);
            dbg!(sandwich);

            wait_randomly(800);
        }

        // Say goodbye!
        self.end_order(&mut server).await?;

        thread::sleep(Duration::from_millis(1000));

        Ok(())
    }

    /// Say the given phrase out loud, display the given sandwich, and send both to
    /// another machine with the given stream.
    async fn say_phrase(
        &self,
        phrase: &str,
        sandwich: Option<Sandwich>,
        stream: &mut TcpStream,
    ) -> anyhow::Result<()> {
        let mut buf = [0; 512];
        bincode::serialize_into(&mut buf as &mut [u8], &sandwich)?;

        // Convert phrase to subtitles!
        self.lang.render(Render {
            ingredients: sandwich.map(|x| x.ingredients).unwrap_or_default(),
            subtitles: self.parse(phrase).unwrap().subtitles(),
        })?;

        // Play the phrase out loud.
        audio::play_phrase(phrase)?;

        // Send the other our words.
        let mut str_buf = [0; 512];
        bincode::serialize_into(&mut str_buf as &mut [u8], &phrase)?;
        stream.write(&str_buf).await?;

        // Send the sandwich.
        stream.write(&buf).await?;

        Ok(())
    }

    pub fn respond(&mut self, input: &str) -> (String, Option<Sandwich>) {
        let sentence = self.parse(input).unwrap();
        let (response, sandwich, next_state) =
            self.state
                .respond(&sentence, &self.lang, &mut *self.encoder);
        if let Some(next) = next_state {
            self.state = next;
        }
        (response, sandwich)
    }
    pub fn parse(&self, input: &str) -> Option<PhraseNode> {
        sentence(input.as_bytes(), &self.lang, &*self.encoder)
    }
    pub fn invent_sandwich(&self) -> Sandwich {
        Sandwich::random(&self.lang.dictionary.ingredients, 6)
    }
    pub fn add_behavior(&mut self, b: impl Behavior + 'static) {
        self.behaviors.push(Box::new(b));
    }
    async fn start_order(&mut self, other: &mut TcpStream) -> anyhow::Result<()> {
        let sammich = self.invent_sandwich();
        self.next_index = 0;
        self.sandwich = Some(sammich);
        self.greet(other).await?;
        for b in &self.behaviors {
            b.start();
        }
        Ok(())
    }
    async fn end_order(&mut self, other: &mut TcpStream) -> anyhow::Result<f64> {
        for b in &self.behaviors {
            b.end();
        }
        let score = self
            .greet(other)
            .await?
            .map(|x| self.judge_sandwich(&x))
            .unwrap_or(0.0);
        println!("sandwich score: {}", score);
        self.sandwich = None;
        Ok(score)
    }
    async fn greet(&self, other: &mut TcpStream) -> anyhow::Result<Option<Sandwich>> {
        let (hello, _) = self
            .lang
            .dictionary
            .first_word_in_class(WordFunction::Greeting);

        self.say_phrase(hello, None, other).await?;

        // And wait for a response!
        let resp: String = {
            let mut buf = [0; 512];
            other.read(&mut buf).await?;
            bincode::deserialize(&buf).unwrap()
        };
        let sandwich: Option<Sandwich> = {
            let mut buf = [0; 512];
            other.read(&mut buf).await?;
            bincode::deserialize(&buf).unwrap()
        };

        println!("{}", resp);
        Ok(sandwich)
    }
    pub fn next_phrase(&mut self) -> Option<String> {
        let sandwich = self.sandwich.as_ref().unwrap();

        let mut next_ingredient = Some(self.next_index);
        // Allow behavior to change what the next ingredient might be.
        for b in &mut self.behaviors {
            next_ingredient = b.next_ingredient(sandwich, next_ingredient);
        }

        if let Some(idx) = next_ingredient {
            if idx >= sandwich.ingredients.len() {
                return None;
            }
            let result = Some(self.encoder.encode(
                &self.lang,
                PositionedIngredient {
                    sandwich,
                    index: idx,
                    history: &self.history[..],
                },
            ));
            self.history.push(idx);
            self.next_index = self.history.iter().max().unwrap_or(&0) + 1;
            result
        } else {
            None
        }
    }

    /// Returns a score for the match between the sandwich we wanted and the sandwich we got.
    /// TODO A low enough score may warrant revisions, depending on how shy this client is.
    pub fn judge_sandwich(&self, result: &Sandwich) -> f64 {
        // For now, just count the number of ingredients that match.
        // TODO Count the number of matching *morphemes*.
        if let Some(sandwich) = &self.sandwich {
            // Number of correct ingredients we did ask for.
            let measure = LevenshteinDamerau::new(1, 1, 1, 1);
            let alignment = measure.align(&result.ingredients, &sandwich.ingredients);
            1.0 / (alignment.distance() + 1) as f64
        } else {
            0.0
        }
    }
}
