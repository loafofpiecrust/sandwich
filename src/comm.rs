use async_std::io;
use async_std::net::{TcpListener, TcpStream};
use hostname;
use rand::prelude::*;
use std::time::Duration;

const SANDWICH_PORT: u16 = 34222;
const HOSTS: &[&str] = &["sandwich2", "loafofpiecrust"];

pub async fn find_peer() -> std::io::Result<TcpStream> {
    // Try connecting to a random peer until it succeeds.
    let ourselves = hostname::get()?;
    let our_name = ourselves.as_os_str().to_str().unwrap();
    println!("our hostname = {}", our_name);
    loop {
        let host = HOSTS.choose(&mut thread_rng()).unwrap();
        if host.eq_ignore_ascii_case(our_name) {
            continue;
        }

        println!("Attempting connection with {}", host);
        let url = format!("{}.local:{}", host, SANDWICH_PORT);
        let stream = io::timeout(Duration::from_secs(1), TcpStream::connect(url)).await;
        if stream.is_ok() {
            return stream;
        }
    }
}

pub async fn wait_for_peer() -> std::io::Result<TcpStream> {
    let conn = TcpListener::bind(format!("0.0.0.0:{}", SANDWICH_PORT)).await?;
    let (stream, _addr) = conn.accept().await?;
    println!("Client connected!!");
    Ok(stream)
}
