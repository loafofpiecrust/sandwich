use crate::{
    audio,
    behavior::{Behavior, DesireEncoder, Encoder, PositionedIngredient, RelativeEncoder},
    comm,
    display::{setup_display, Render, RenderSender},
    grammar,
    grammar::WordFunction,
    sandwich::Sandwich,
    state::{Idle, OrderingSandwich, State},
    wait_randomly,
};
use async_std::future::timeout;
use async_std::net::TcpStream;
use async_std::prelude::*;
use bincode;
use futures::{pin_mut, select, FutureExt};
use grammar::{sentence, Dictionary, PhraseNode};
use rand::prelude::*;
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
    pub lang: Language,
    encoder: Box<dyn Encoder>,
}
impl Client {
    pub fn new() -> Self {
        Self {
            state: Box::new(Idle),
            behaviors: Vec::new(),
            sandwich: None,
            lang: Language::new(),
            encoder: Box::new(RelativeEncoder::new(0.8, DesireEncoder)),
        }
    }

    pub async fn connect_with_peer(&mut self) -> anyhow::Result<()> {
        let client = comm::find_peer().fuse();
        let server = comm::wait_for_peer().fuse();
        pin_mut!(client, server);
        select! {
            s = client => self.server(s?).await,
            s = server => self.server(s?).await,
        }
    }

    async fn server(&mut self, mut stream: TcpStream) -> anyhow::Result<()> {
        // Pick a random timeout for the initial handshake.
        // TODO Influenced by shyness.
        let waiting_time = thread_rng().gen_range(300, 1500);
        println!("Waiting {}ms before ordering", waiting_time);
        let res = timeout(
            Duration::from_millis(waiting_time),
            self.single_step(&mut stream, false),
        )
        .await;

        // If we don't hear anything from the other side, initiate with our own greeting.
        let our_order = res.is_err();
        if our_order {
            println!("requesting an order!");
            self.start_order(&mut stream).await?;
        }

        while {
            timeout(
                Duration::from_millis(3000),
                self.single_step(&mut stream, false),
            )
            .await??
        } {}

        if our_order {
            println!("ending the order");
            self.end_order(&mut stream).await?;
        }
        thread::sleep(Duration::from_millis(1000));

        Ok(())
    }

    /// Returns whether to keep going (true), or if the order is over (false).
    async fn single_step(
        &mut self,
        stream: &mut TcpStream,
        skip_input: bool,
    ) -> anyhow::Result<bool> {
        let (request, sandwich) = if skip_input {
            (None, None)
        } else {
            // Wait for a request,
            let request: String = {
                let mut buf = [0; 512];
                stream.read(&mut buf).await?;
                dbg!(bincode::deserialize(&buf)?)
            };
            let sandwich: Option<Sandwich> = {
                let mut buf = [0; 512];
                stream.read(&mut buf).await?;
                bincode::deserialize(&buf)?
            };
            println!("Received from the other side! {}", request);
            println!("Sandwich: {:?}", sandwich);
            (Some(request), sandwich)
        };

        // Then respond with words and maybe a sandwich.
        let (resp, sandwich) = self.respond(request.as_ref().map(|x| x as &str), sandwich.as_ref());
        if let Some(resp) = resp {
            let cont = sandwich.is_none();
            self.say_phrase(&resp, sandwich, stream).await?;
            Ok(cont)
        } else {
            Ok(false)
        }
    }

    /// Say the given phrase out loud, display the given sandwich, and send both to
    /// another machine with the given stream.
    async fn say_phrase(
        &self,
        phrase: &str,
        sandwich: Option<Sandwich>,
        stream: &mut TcpStream,
    ) -> anyhow::Result<()> {
        println!("saying {}", phrase);
        println!("with sandwich {:?}", sandwich);

        let mut buf = [0; 512];
        bincode::serialize_into(&mut buf as &mut [u8], &sandwich)?;

        // Convert phrase to subtitles!
        self.lang.render(Render {
            ingredients: sandwich.map(|x| x.ingredients),
            subtitles: self.parse(phrase).map(|x| x.subtitles()),
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

    pub fn respond(
        &mut self,
        input: Option<&str>,
        sandwich: Option<&Sandwich>,
    ) -> (Option<String>, Option<Sandwich>) {
        let sentence = input
            .and_then(|i| self.parse(i))
            .unwrap_or(PhraseNode::Empty);
        let (response, sandwich, next_state) = self.state.respond(
            &sentence,
            sandwich,
            &self.lang,
            &mut *self.encoder,
            &mut self.behaviors,
        );
        if let Some(next) = next_state {
            println!("Transitioning to {:?}", next);
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
        // TODO Use encoder for "I want sandwich" => "ku nu"
        self.say_phrase("ku nu", None, other).await?;
        self.state = Box::new(OrderingSandwich::new(&self.lang.dictionary.ingredients));
        for b in &self.behaviors {
            b.start();
        }
        Ok(())
    }
    async fn end_order(&mut self, other: &mut TcpStream) -> anyhow::Result<()> {
        for b in &self.behaviors {
            b.end();
        }
        let sandwich = self.greet(other).await?;
        self.state.respond(
            &PhraseNode::Empty,
            sandwich.as_ref(),
            &self.lang,
            &mut *self.encoder,
            &mut self.behaviors,
        );
        // self.sandwich = None;
        Ok(())
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
        println!("Received sandwich: {:?}", sandwich);
        Ok(sandwich)
    }
    // pub fn next_phrase(&mut self) -> Option<String> {
    //     let sandwich = self.sandwich.as_ref().unwrap();

    //     let mut next_ingredient = Some(self.next_index);
    //     // Allow behavior to change what the next ingredient might be.
    //     for b in &mut self.behaviors {
    //         next_ingredient = b.next_ingredient(sandwich, next_ingredient);
    //     }

    //     if let Some(idx) = next_ingredient {
    //         if idx >= sandwich.ingredients.len() {
    //             return None;
    //         }
    //         let result = Some(self.encoder.encode(
    //             &self.lang,
    //             PositionedIngredient {
    //                 sandwich,
    //                 index: idx,
    //                 history: &self.history[..],
    //             },
    //         ));
    //         self.history.push(idx);
    //         self.next_index = self.history.iter().max().unwrap_or(&0) + 1;
    //         result
    //     } else {
    //         None
    //     }
    // }
}
