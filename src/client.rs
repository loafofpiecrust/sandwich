use crate::{
    audio,
    behavior::{
        ops, Behavior, DesireEncoder, Encoder, Message, Operation, Order, Personality,
        RelativeEncoder,
    },
    comm,
    display::{setup_display, Render, RenderSender},
    grammar,
    grammar::AnnotatedPhrase,
    grammar::FullParse,
    grammar::WordFunction,
    sandwich::Sandwich,
    state::{Idle, OrderingSandwich, State},
};
use async_std::future::timeout;
use async_std::io::BufReader;
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::sync::{Arc, RwLock};
use async_std::task;
use bincode;
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::sink::SinkExt;
use itertools::Itertools;
use rand::prelude::*;
use take_mut::take;
// use futures::prelude::*;
use futures::{pin_mut, select, FutureExt};
use grammar::{prob_sentence_new, Dictionary, PhraseNode};
use std::{thread, time::Duration};

pub struct Client {
    /// We'll have a few words with default parts of speech if totally ambiguous.
    pub state: Box<dyn State>,
    behaviors: Vec<Box<dyn Behavior>>,
    pub sandwich: Option<Sandwich>,
    pub lang: Personality,
    encoder: Box<dyn Encoder>,
    last_result: Sandwich,
}
impl Client {
    pub fn new() -> Self {
        Self {
            state: Box::new(Idle),
            behaviors: Vec::new(),
            sandwich: None,
            // Make a new personality if there's none saved.
            lang: Personality::load().unwrap_or_else(|_| Personality::new()),
            encoder: Box::new(RelativeEncoder::new(0.8, DesireEncoder)),
            last_result: Sandwich::default(),
        }
    }

    pub async fn connect_with_peer(&mut self) -> anyhow::Result<()> {
        // Keep doing sandwich interactions forever.
        loop {
            let client = comm::find_peer().fuse();
            let server = comm::wait_for_peer().fuse();
            pin_mut!(client, server);
            let r = select! {
                s = client => self.new_customer(s.0?, s.1).await,
                s = server => self.new_server(s.0?, s.1).await,
            };
            r?;
        }
    }

    async fn receives_msgs(mut stream: TcpStream, mut chan: Sender<Message>) -> anyhow::Result<()> {
        println!("Receiving!");
        loop {
            chan.send(Message::recv(&mut stream).await?).await?;
        }
    }

    async fn new_customer(
        &mut self,
        mut stream: TcpStream,
        color: &'static str,
    ) -> anyhow::Result<()> {
        let mut rng = thread_rng();

        // Set the shared background color.
        self.lang.render(Render {
            ingredients: None,
            subtitles: None,
            background: Some(color),
        })?;

        // No greeting for now, treating the TCP connection itself as the greeting.
        let mut order = Order::new(&self.lang);
        let (msg_sx, mut msg_rx) = channel(1);
        let recv_task = task::spawn(Self::receives_msgs(stream.clone(), msg_sx));
        loop {
            // Refill the ingredient inventory when we get really low on
            // *everything*. So we could run out of several things before
            // hitting the reset.
            if self.lang.total_inventory_count() < 10 {
                self.lang.reset_inventory();
            }

            // Save our personality frequently.
            self.lang.save()?;

            // TODO Handle the Err case here by breaking the loop.
            // TODO Add timeout to this instead of statically waiting at the end
            // of every iteration. That'll make this more responsive. Then,
            // politeness extending the timeout makes real world sense, rather
            // than a polite machine waiting for seconds even after the request
            // is fulfilled.
            if let Ok(msg) = msg_rx.try_next() {
                println!("received {:?}", msg);
                if let Some(sandwich) = msg.and_then(|m| m.sandwich) {
                    self.last_result = sandwich;
                }
            }

            // If our last operation succeeded, learn from that experience.
            if order.last_op_successful(&mut self.lang, &self.last_result) {
                if let Some(op) = order.last_op() {
                    self.lang.apply_upgrade(op.skills());
                }
                // Tell our server that they're doing a good job!
                self.say_and_send(&mut stream, &ops::Affirm, None).await?;
            }

            // Send over the next operation!
            let op = order.pick_op(&self.lang, &self.last_result);

            if let Some(mut op) = op {
                // Request two operations at once if planned and not shy.
                if rng.gen_bool(self.lang.planned)
                    && !rng.gen_bool(self.lang.shyness)
                    && rng.gen_bool(self.lang.conjunction)
                {
                    let assumed_sandwich = op.apply(self.last_result.clone(), &mut self.lang);
                    if let Some(next_op) = order.pick_op(&self.lang, &assumed_sandwich) {
                        op = Box::new(ops::Compound(op, next_op));
                    }
                }
                println!("op: {:?}", op);
                self.say_and_send(&mut stream, &*op, None).await?;
                // Send this operation to our history box.
                order.archive(op);
                // Wait some time between each of our requests.
                // TODO Some machines may wait for responses before sending the
                // next operation. Or start waiting if there's a buffer of
                // messages that haven't been acknowledged.
                // TODO Change wait time based on shyness/politeness
                task::sleep(Duration::from_millis(rng.gen_range(700, 1500))).await;
            } else {
                // Break the loop if there's no more operations to make!
                println!("the sandwich is finished!");
                break;
            }
        }
        // Say thank you and goodbye.
        self.say_and_send(&mut stream, &ops::Finish, None).await?;
        Ok(())
    }

    async fn say_and_send(
        &self,
        stream: &mut TcpStream,
        op: &dyn Operation,
        sandwich: Option<Sandwich>,
    ) -> anyhow::Result<()> {
        let s = op.encode(&self.lang);
        self.say_phrase(&s, None).await?;
        let message = Message::new(Some(s.to_string()), None);
        message.send(stream).await?;
        Ok(())
    }

    // async fn say_and_send(
    //     &self,
    //     op: Box<dyn Operation>,
    //     stream: &mut TcpStream,
    // ) -> anyhow::Result<()> {
    //     let s = op.encode(&self.lang);
    //     self.say_phrase(&s, None);
    //     let msg = Message::new(Some(s), None);
    //     msg.send(&mut stream).await?;
    //     Ok(())
    // }

    async fn new_server(
        &mut self,
        mut stream: TcpStream,
        color: &'static str,
    ) -> anyhow::Result<()> {
        let mut rng = thread_rng();

        // Set the shared background color.
        self.lang.render(Render {
            ingredients: None,
            subtitles: None,
            background: Some(color),
        })?;

        let mut persistent_ops = Vec::<Box<dyn Operation>>::new();
        self.last_result = Sandwich::default();
        // Only break the loop when the order is complete.
        while !self.last_result.complete {
            // Save our personality frequently.
            self.lang.save()?;

            // TODO This machine might wait to receive multiple operations before applying them all at once.
            let msg = Message::recv(&mut stream).await?;

            if let Some(FullParse {
                operation: mut op,
                lang: lang_change,
                lex,
            }) = self.parse(&msg.text.unwrap())
            {
                // Save the lex of this phrase for one turn.
                // If we receive a positive reply from the client machine, use
                // this lex to update our word association weights.
                // TODO Initial shared vocab should just be Yes + No i guess?
                // TODO Check if `op` is an affirmation, in which case use last_lex!
                self.lang.last_lex = Some(lex);

                // Apply all persistent operations at every turn.
                for passive_op in &persistent_ops {
                    self.last_result = passive_op.apply(self.last_result.clone(), &mut self.lang);
                }

                // If spite is high enough, do the opposite of their order.
                if rng.gen_bool(self.lang.spite) {
                    op = op.reverse();
                    // Feel the release of anger calm you.
                    self.lang.spite = 0.0;
                }

                // Apply the operation to our sandwich.
                self.last_result = op.apply(self.last_result.clone(), &mut self.lang);
                self.lang.apply_upgrade(lang_change);

                if op.is_persistent() {
                    persistent_ops.push(op);
                }

                if let Some(op) = self.pick_server_op() {
                    let s = op.encode(&self.lang);
                    self.say_phrase(&s, Some(self.last_result.clone())).await?;
                    let message = Message::new(Some(s.to_string()), None);
                    message.send(&mut stream).await?;
                }

                // TODO Say response too? Render upon saying a response?
                // self.lang.render(Render {
                //     subtitles: None,
                //     ingredients: Some(self.last_result.ingredients.clone()),
                //     background: None,
                // })?;

                // Send the current sandwich status back over!
                let new_msg = Message::new(None, Some(self.last_result.clone()));
                new_msg.send(&mut stream).await?;
            } else {
                println!("Failed to parse phrase")
            }
        }

        println!("The order is finished!");
        Ok(())
    }

    fn pick_server_op(&self) -> Option<Box<dyn Operation>> {
        // TODO Say NeverAdd(it) if we don't have a requested ingredient.
        None
    }

    /// Say the given phrase out loud, display the given sandwich, and send both to
    /// another machine with the given stream.
    async fn say_phrase(
        &self,
        phrase: &str,
        sandwich: Option<Sandwich>,
        // stream: &mut TcpStream,
    ) -> anyhow::Result<()> {
        println!("saying {}", phrase);
        // println!("{:?}", self.parse(phrase));
        println!("with sandwich {:?}", sandwich);

        // let mut buf = [0; 512];
        // bincode::serialize_into(&mut buf as &mut [u8], &sandwich)?;

        // Convert phrase to subtitles!
        self.lang.render(Render {
            ingredients: sandwich.map(|x| x.ingredients),
            // subtitles: self.parse(phrase).map(|x| x.subtitles()),
            subtitles: self
                .lex(phrase)
                .map(|w| w.into_iter().map(|w| w.entry.unwrap().definition).join(" ")),
            background: None,
        })?;

        // Play the phrase out loud.
        audio::play_phrase(phrase)?;

        // Send the other our words.
        // let mut str_buf = [0; 512];
        // bincode::serialize_into(&mut str_buf as &mut [u8], &phrase)?;
        // stream.write(&str_buf).await?;

        // // Send the sandwich.
        // stream.write(&buf).await?;

        Ok(())
    }

    pub fn respond(
        &mut self,
        input: Option<&str>,
        sandwich: Option<&Sandwich>,
    ) -> (Option<String>, Option<Sandwich>) {
        todo!()
        // let sentence = input
        //     .and_then(|i| self.parse(i))
        //     .unwrap_or(PhraseNode::Empty);
        // let (response, sandwich, next_state) = self.state.respond(
        //     &sentence,
        //     sandwich,
        //     &self.lang,
        //     &mut *self.encoder,
        //     &mut self.behaviors,
        // );
        // if let Some(next) = next_state {
        //     println!("Transitioning to {:?}", next);
        //     self.state = next;
        // }
        // (response, sandwich)
    }
    pub fn parse(&mut self, input: &str) -> Option<FullParse> {
        prob_sentence_new(input.as_bytes(), &self.lang)
    }
    pub fn lex(&self, input: &str) -> Option<Vec<grammar::AnnotatedWord>> {
        grammar::phrase(input.as_bytes())
            .ok()
            .map(|p| grammar::annotate(p.1, &self.lang))
    }
    pub fn add_behavior(&mut self, b: impl Behavior + 'static) {
        self.behaviors.push(Box::new(b));
    }
    async fn start_order(&mut self, other: &mut TcpStream) -> anyhow::Result<()> {
        // TODO Use encoder for "I want sandwich" => "ku nu"
        // self.say_phrase("ku nu", None, other).await?;
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
        let (hello, _) = self.lang.dictionary.word_for_def(WordFunction::Greeting);

        // self.say_phrase(hello, None, other).await?;

        // And wait for a response!
        let resp: String = {
            let mut buf = [0; 512];
            other.read(&mut buf).await?;
            bincode::deserialize(&buf)?
        };
        let sandwich: Option<Sandwich> = {
            let mut buf = [0; 512];
            other.read(&mut buf).await?;
            bincode::deserialize(&buf)?
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
