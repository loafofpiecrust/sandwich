use crate::{
    audio,
    behavior::{
        ops, Behavior, DispatchMessage, Encoder, Event, Message, Operation, Order, Personality,
    },
    comm,
    display::{PersonalityAction, Render},
    grammar,
    grammar::FullParse,
    sandwich::{Ingredient, Sandwich},
    state::{Idle, OrderingSandwich, State},
};
use async_std::future::timeout;
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::sync::{Arc, RwLock};
use async_std::task;
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::sink::SinkExt;
use itertools::Itertools;
use piston_window::{Button, Key};
use rand::prelude::*;
// use futures::prelude::*;
use futures::{pin_mut, select, FutureExt};
use grammar::{sentence_new, Dictionary, PhraseNode};
use std::{
    thread,
    time::{Duration, Instant},
};

pub struct Client {
    /// We'll have a few words with default parts of speech if totally ambiguous.
    pub state: Box<dyn State>,
    behaviors: Vec<Box<dyn Behavior>>,
    pub lang: Personality,
    // encoder: Box<dyn Encoder>,
    last_result: Sandwich,
}
impl Client {
    pub fn new() -> Self {
        Self {
            state: Box::new(Idle),
            behaviors: Vec::new(),
            // Make a new personality if there's none saved.
            lang: Personality::load().unwrap_or_else(|_| Personality::new()),
            // encoder: Box::new(RelativeEncoder::new(0.8, DesireEncoder)),
            last_result: Sandwich::default(),
        }
    }

    pub async fn connect_with_peer(&mut self) -> anyhow::Result<()> {
        let mut rng = thread_rng();
        // Keep doing sandwich interactions forever.
        // Rotate between trying to be a customer and trying to be a server.

        let mut actions = Self::connect_to_central_dispatch().await;

        loop {
            // TODO Allow actions to apply *during* an order too.
            while let Ok(Some(action)) = actions.try_next() {
                action(&mut self.lang);
            }

            // Clear the display.
            self.lang.render(Render::clear())?;

            // Either be a client or server.
            let dur = Duration::from_millis(rng.gen_range(800, 2000));
            if rng.gen_bool(0.5) {
                if let Ok(c) = timeout(dur, comm::find_peer()).await {
                    dbg!(self.new_customer(c.0?, c.1).await);
                }
            } else {
                if let Ok(c) = timeout(dur, comm::wait_for_peer()).await {
                    dbg!(self.new_server(c.0?, c.1).await);
                }
            }
        }
    }

    async fn receives_msgs(mut stream: TcpStream, mut chan: Sender<Message>) -> anyhow::Result<()> {
        println!("Receiving!");
        loop {
            chan.send(Message::recv(&mut stream).await?).await?;
        }
    }

    async fn eat_sandwich(&mut self, sandwich: Sandwich) -> anyhow::Result<()> {
        // Eat the sandwich ingredient by ingredient.
        // Alternate between background colors.
        let mut ingredients = sandwich.ingredients.clone();
        let mut color_alt = false;
        while !ingredients.is_empty() {
            self.lang.render(Render {
                ingredients: Some(ingredients.clone()),
                subtitles: Some(String::new()),
                background: Some(if color_alt { "000000ff" } else { "ffffffff" }),
            })?;

            // Savor the sandwich!
            task::sleep(Duration::from_millis(800)).await;

            let top = ingredients.pop();
            color_alt = !color_alt;

            // If we're allergic to this ingredient, we might have a reaction.
            if let Some(top) = top {
                if self.lang.allergic_reaction(&top) {
                    self.have_seizure(top).await?;
                    self.death_and_rebirth().await?;
                    break;
                }
            }
        }
        // Now eat the sandwich, and save in our history.
        self.lang.eat(sandwich);
        Ok(())
    }

    async fn death_and_rebirth(&mut self) -> anyhow::Result<()> {
        self.lang.render(Render::clear())?;
        self.lang = Personality::new();
        task::sleep(Duration::from_millis(1500)).await;
        Ok(())
    }

    async fn have_seizure(&mut self, allergen: Ingredient) -> anyhow::Result<()> {
        // Show just the ingredient we're reacting to.
        self.lang.render(Render {
            ingredients: Some(vec![allergen]),
            subtitles: None,
            background: None,
        })?;
        let flicker_gap = Duration::from_millis(100);
        let total_flickers: u32 = 2500 / 100;
        for i in 0..total_flickers {
            // Even numbered time-steps use a different color, creating a
            // flashing effect.
            let color_alt = i % 2 == 0;
            self.lang.render(Render {
                ingredients: None,
                subtitles: None,
                background: Some(if color_alt { "000000ff" } else { "ffffffff" }),
            })?;
            task::sleep(flicker_gap).await;
        }
        Ok(())
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

        println!("desired sandwich: {:?}", order.desired);

        let (msg_sx, mut msg_rx) = channel(1);
        let recv_task = task::spawn(Self::receives_msgs(stream.clone(), msg_sx));
        let mut failed_attempts = 0;
        loop {
            if failed_attempts > 8 {
                // Give up on the sandwich...
                break;
            }
            // Stress modifier multiplies value intesities, shortens wait times, etc.
            let stress = self.lang.stress();

            // End any finished events.
            if let Some(evt) = self.lang.event.as_ref() {
                if evt.is_over() {
                    self.lang.event = None;
                }
            }

            // while let Ok(action) = self.lang.display.actions.try_recv() {
            //     action(&mut self.lang);
            // }

            // Save our personality frequently.
            self.lang.save()?;

            // TODO Handle the Err case here by breaking the loop.
            // TODO Add timeout to this instead of statically waiting at the end
            // of every iteration. That'll make this more responsive. Then,
            // politeness extending the timeout makes real world sense, rather
            // than a polite machine waiting for seconds even after the request
            // is fulfilled.
            // Wait some time between each of our requests.
            // TODO Some machines may wait for responses before sending the
            // next operation. Or start waiting if there's a buffer of
            // messages that haven't been acknowledged.
            let min_wait = (200.0 * self.lang.shyness * 10.0 / stress) as u64;
            let wait_time = Duration::from_millis(rng.gen_range(
                min_wait,
                (800.0 * self.lang.politeness * 10.0 / stress) as u64,
            ));
            task::sleep(wait_time).await;
            while let Ok(Some(msg)) = msg_rx.try_next() {
                if let Some(sandwich) = msg.sandwich {
                    println!("received {}", sandwich);
                    self.last_result = sandwich;
                }

                // If the server sent back any changes to our order, like them
                // being out of an ingredient, apply that to our desired sandwich.
                if let Some(FullParse { operation, lex, .. }) =
                    msg.text.and_then(|t| self.parse(&t))
                {
                    println!("Received response op: {:?}", operation);
                    order.desired = operation.apply(order.desired.clone(), &mut self.lang);
                    self.lang.last_lex = Some(lex);

                    // If we asked a question that caused a change in our
                    // sandwich, affirm that we understood it.
                    if order.last_question_failed(&mut self.lang, &self.last_result) {
                        order.desired = ops::Affirm.apply(order.desired.clone(), &mut self.lang);
                    }
                }
            }

            // If our last operation succeeded, learn from that experience.
            if order.last_op_successful(&mut self.lang, &self.last_result) {
                if let Some(op) = order.last_op() {
                    self.lang.apply_upgrade(op.skills());
                }
                // Tell our server that they're doing a good job!
                self.say_and_send(&mut stream, Some(&ops::Affirm), None)
                    .await?;
            } else {
                failed_attempts += 1;
            }

            // Send over the next operation!
            let op = order.pick_op(&self.lang, &self.last_result);

            if let Some(mut op) = op {
                // Request two operations at once if planned and not shy.
                if rng.gen_bool((self.lang.planned * stress).min(0.95))
                    && !rng.gen_bool(self.lang.shyness / stress)
                    && rng.gen_bool(self.lang.conjunction)
                {
                    let assumed_sandwich = op.apply(self.last_result.clone(), &mut self.lang);
                    if let Some(next_op) = order.pick_op(&self.lang, &assumed_sandwich) {
                        op = Box::new(ops::Compound(op, next_op));
                    }
                }
                println!("op: {:?}", op);
                self.say_and_send(&mut stream, Some(&*op), None).await?;
                // Send this operation to our history box.
                order.archive(op);
            } else {
                // Break the loop if there's no more operations to make!
                println!("the sandwich is finished!");
                break;
            }
        }
        // Say thank you and goodbye.
        self.say_and_send(&mut stream, Some(&ops::Finish), None)
            .await?;
        if let Some(sandwich) = order.last_result {
            self.eat_sandwich(sandwich).await?;
        }
        Ok(())
    }

    async fn say_and_send(
        &self,
        stream: &mut TcpStream,
        op: Option<&dyn Operation>,
        sandwich: Option<Sandwich>,
    ) -> anyhow::Result<()> {
        // TODO Save this encoding as the last lex of our own phrase.
        let phrase = op.map(|op| op.encode(&self.lang));
        let s = phrase.map(|phrase| phrase.into_iter().map(|x| x.word.to_string()).join(" "));
        self.say_phrase(s.as_deref(), sandwich.clone()).await?;
        let message = Message::new(s.to_owned(), sandwich);
        dbg!(&message);
        message.send(stream).await?;
        Ok(())
    }

    pub async fn central_dispatch(&self) -> anyhow::Result<()> {
        let mut exclusive_host = None;
        println!("running central dispatch");
        // Connect to all sandwich machines.
        let mut connections = comm::central_dispatch().await;
        // Then accept real-time events from the window...
        while let Ok(key) = self.lang.display.keys.recv() {
            match key {
                Button::Keyboard(Key::D1) => exclusive_host = Some(&comm::HOSTS[0]),
                Button::Keyboard(Key::D2) => exclusive_host = Some(&comm::HOSTS[1]),
                Button::Keyboard(Key::D3) => exclusive_host = Some(&comm::HOSTS[2]),
                Button::Keyboard(Key::D4) => exclusive_host = Some(&comm::HOSTS[3]),
                Button::Keyboard(Key::D5) => exclusive_host = Some(&comm::HOSTS[4]),
                Button::Keyboard(Key::D6) => exclusive_host = Some(&comm::HOSTS[5]),
                Button::Keyboard(Key::D0) => exclusive_host = None,
                _ => {
                    // ...and dispatch them.
                    // For now, all key codes to all clients.
                    for (host, stream) in &mut connections {
                        let matches = exclusive_host.map(|h| host == h).unwrap_or(true);
                        if matches {
                            println!("sending {:?} to {}", key, host);
                            DispatchMessage::new(key).send(stream).await?;
                        }
                    }
                }
            };
        }
        Ok(())
    }

    async fn connect_to_central_dispatch() -> Receiver<PersonalityAction> {
        let (mut sx, rx) = channel::<PersonalityAction>(1);
        let mut connection = comm::wait_for_central_dispatch()
            .await
            .expect("Couldn't connect to central dispatch");
        task::spawn(async move {
            loop {
                if let Ok(msg) = DispatchMessage::recv(&mut connection).await {
                    match msg.key {
                        Button::Keyboard(Key::A) => {
                            sx.send(|p| {
                                println!("AVOCADO!!");
                                p.increase_preference("avocado")
                            })
                            .await;
                        }
                        Button::Keyboard(Key::E) => {
                            sx.send(|p| p.increase_preference("fried-egg")).await;
                        }
                        Button::Keyboard(Key::S) => {
                            sx.send(|p| p.spite += 0.1).await;
                        }
                        Button::Keyboard(Key::R) => {
                            sx.send(|p| p.event = Some(Event::LunchRush(Instant::now())))
                                .await;
                        }
                        _ => {}
                    }
                }
            }
        });
        rx
    }

    async fn new_server(
        &mut self,
        mut stream: TcpStream,
        color: &'static str,
    ) -> anyhow::Result<()> {
        let mut rng = thread_rng();

        // Refill the ingredient inventory when we get really low on
        // *everything*. So we could run out of several things before
        // hitting the reset.
        if self.lang.total_inventory_count() < 10 {
            self.lang.reset_inventory();
        }

        // Set the shared background color.
        self.lang.render(Render {
            ingredients: None,
            subtitles: None,
            background: Some(color),
        })?;

        let mut order = Order::new(&self.lang);
        self.last_result = Sandwich::default();
        // Only break the loop when the order is complete.
        while !self.last_result.complete {
            // Stress modifier multiplies value intesities, shortens wait times, etc.
            let stress = self.lang.stress();

            // End any finished events.
            if let Some(evt) = self.lang.event.as_ref() {
                if evt.is_over() {
                    self.lang.event = None;
                }
            }

            // If there's been user interaction, make sure to apply the results.
            // while let Ok(Some(action)) = actions.try_next() {
            //     action(&mut self.lang);
            // }

            // Save our personality frequently.
            self.lang.save()?;

            // TODO This machine might wait to receive multiple operations before applying them all at once.
            let msg = timeout(Duration::from_secs(5), Message::recv(&mut stream)).await??;

            // If there are zeroes, we might parse as a (None, None) accidentally.
            // So let's check for that.
            if msg.text.is_none() && msg.sandwich.is_none() {
                println!("Received a completely empty message");
                // break;
            }

            if let Some(FullParse {
                operation: mut op,
                lang: lang_change,
                lex,
            }) = msg.text.and_then(|t| self.parse(&t))
            {
                // Apply all persistent operations at every turn.
                for passive_op in &order.persistent_ops {
                    self.last_result = passive_op.apply(self.last_result.clone(), &mut self.lang);
                }

                // If spite is high enough, do the opposite of their order.
                if rng.gen_bool((self.lang.spite * stress).min(0.99)) {
                    op = op.reverse();
                    // Feel the release of anger calm you.
                    self.lang.spite = 0.0;
                }

                // Apply the operation to our sandwich.
                self.last_result = op.apply(self.last_result.clone(), &mut self.lang);
                self.lang.apply_upgrade(lang_change);
                let resp = op.respond(&self.lang);
                self.say_and_send(
                    &mut stream,
                    resp.as_ref().map(|x| &**x),
                    Some(self.last_result.clone()),
                )
                .await?;

                if op.is_persistent() {
                    order.persistent_ops.push(op);
                }

                // Save the lex of this phrase for one turn.
                // If we receive a positive reply from the client machine, use
                // this lex to update our word association weights.
                // TODO Initial shared vocab should just be Yes + No i guess?
                // TODO Check if `op` is an affirmation, in which case use last_lex!
                println!("lexed {:?}", lex);
                self.lang.last_lex = Some(lex);
            } else {
                println!("Failed to parse phrase")
            }
        }

        println!("The order is finished!");
        Ok(())
    }

    /// Say the given phrase out loud, display the given sandwich, and send both to
    /// another machine with the given stream.
    async fn say_phrase(
        &self,
        phrase: Option<&str>,
        sandwich: Option<Sandwich>,
        // stream: &mut TcpStream,
    ) -> anyhow::Result<()> {
        println!("saying {:?}", phrase);
        // println!("{:?}", self.parse(phrase));
        println!("with sandwich {:?}", sandwich);

        // let mut buf = [0; 512];
        // bincode::serialize_into(&mut buf as &mut [u8], &sandwich)?;

        // Convert phrase to subtitles!
        self.lang.render(Render {
            ingredients: sandwich.map(|x| x.ingredients),
            // Always render a string, so that the current subtitles go away
            // next time we say/do anything.
            subtitles: Some(
                phrase
                    .as_ref()
                    .and_then(|phrase| {
                        self.lex(phrase)
                            .map(|w| w.into_iter().map(|w| w.entry.unwrap().definition).join(" "))
                    })
                    .unwrap_or(String::new()),
            ),
            background: None,
        })?;

        // Play the phrase out loud.
        if let Some(p) = phrase {
            audio::play_phrase(p, self.lang.pitch_shift)?;
        }

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
        sentence_new(input.as_bytes(), &self.lang)
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
}
