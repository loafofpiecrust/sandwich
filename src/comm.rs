use crate::behavior::{self, Event, Operation};
use async_std::io;
use async_std::net::{TcpListener, TcpStream};
use hostname;
use lazy_static::*;
use maplit::*;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

const SANDWICH_PORT: u16 = 34222;
const DISPATCH_PORT: u16 = 34223;
pub const HOSTS: &[&str] = &[
    "sandwich1",
    "sandwich2",
    "sandwich3",
    "sandwich4",
    "sandwich5",
    "sandwich6",
];
const DISPATCH_HOST: &str = "loafofpiecrust";
lazy_static! {
    pub static ref BG_COLORS: HashMap<&'static str, &'static str> = hashmap! {
        "sandwich1" => "44000dff",
        "sandwich2" => "ffbd33ff",
        "sandwich3" => "00000000",
        "sandwich4" => "148342ff",
        "sandwich5" => "2c5182ff",
        "sandwich6" => "00000000",
        "loafofpiecrust" => "3ca59dff",
    };
}

pub fn is_dispatch_host() -> bool {
    let ourselves = hostname::get().expect("We should have a hostname");
    let our_name = ourselves.as_os_str().to_str().unwrap();
    our_name == DISPATCH_HOST
}

pub async fn find_peer() -> (std::io::Result<TcpStream>, &'static str) {
    // Try connecting to a random peer until it succeeds.
    let ourselves = hostname::get().expect("We should have a hostname");
    let our_name = ourselves.as_os_str().to_str().unwrap();
    println!("our hostname = {}", our_name);
    loop {
        let host = HOSTS.choose(&mut thread_rng()).unwrap();
        if host.eq_ignore_ascii_case(our_name) {
            continue;
        }

        println!("Attempting connection with {}", host);
        let url = format!("{}.local:{}", host, SANDWICH_PORT);
        let stream = io::timeout(Duration::from_millis(300), TcpStream::connect(url)).await;
        if stream.is_ok() {
            println!("Connected to {}", host);
            return (stream, BG_COLORS[host]);
        }
    }
}

pub async fn wait_for_peer() -> (std::io::Result<TcpStream>, &'static str) {
    let conn = TcpListener::bind(format!("0.0.0.0:{}", SANDWICH_PORT))
        .await
        .expect("Failed to start TCP server");
    let (stream, _addr) = conn.accept().await.expect("Failed to find peer");
    println!("Client connected!!");
    let ourselves = hostname::get().expect("We should have a hostname");
    let hostname = ourselves.as_os_str().to_str().unwrap().to_lowercase();
    (Ok(stream), BG_COLORS[&hostname as &str])
}

pub async fn wait_for_central_dispatch() -> std::io::Result<TcpStream> {
    let conn = TcpListener::bind(format!("0.0.0.0:{}", DISPATCH_PORT)).await?;
    let (stream, _addr) = conn.accept().await?;
    println!("Dispatch connected.");
    Ok(stream)
}

// Returns a map of hostname to the relevant TCP stream.
pub async fn central_dispatch() -> HashMap<&'static str, TcpStream> {
    let ourselves = hostname::get().expect("We should have a hostname");
    let our_name = ourselves.as_os_str().to_str().unwrap();
    let mut result = HashMap::new();
    for host in HOSTS {
        let url = format!("{}.local:{}", host, DISPATCH_PORT);
        println!("Attempting connection with {}", url);
        let stream = io::timeout(Duration::from_millis(1000), TcpStream::connect(url)).await;
        if let Ok(s) = stream {
            result.insert(*host, s);
        }
    }
    result
}

// #[derive(Serialize, Deserialize)]
// pub enum DispatchMessage {
//     Op(Box<dyn Operation>),
//     Event(Event),
// }
