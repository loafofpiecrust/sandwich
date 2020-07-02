mod audio;
mod behavior;
mod client;
mod comm;
mod display;
mod grammar;
mod sandwich;
mod sawtooth;
mod state;

use anyhow;
use client::Client;
use rand::prelude::*;
use std::thread;
use std::time::Duration;

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let mut c = Client::new();
    c.add_behavior(behavior::Forgetful::new(0.3));
    if comm::is_dispatch_host() {
        c.central_dispatch().await
    } else {
        c.connect_with_peer().await
    }
}

pub fn wait_randomly(millis: u64) {
    let (min, max) = (millis / 2, millis * 2);
    thread::sleep(Duration::from_millis(thread_rng().gen_range(min, max)));
}
