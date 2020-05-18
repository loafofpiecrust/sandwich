use async_std::io;
use async_std::net::{TcpListener, TcpStream};
use rand::prelude::*;
use std::time::Duration;

const SANDWICH_PORT: u16 = 34222;

pub async fn find_peer() -> std::io::Result<TcpStream> {
    // Try connecting to a random peer until it succeeds.
    loop {
        let num: usize = thread_rng().gen_range(1, 6);
        println!("Attempting connection with sandwich{}", num);
        let url = format!("sandwich{}.local:{}", num, SANDWICH_PORT);
        let stream = io::timeout(Duration::from_secs(2), TcpStream::connect(url)).await;
        if stream.is_ok() {
            return stream;
        }
    }
}

pub async fn wait_for_peer() -> std::io::Result<TcpStream> {
    let conn = TcpListener::bind(format!("0.0.0.0:{}", SANDWICH_PORT)).await?;
    let (mut stream, addr) = conn.accept().await?;
    println!("Client connected!!");
    config_stream(&mut stream).await?;
    Ok(stream)
}

async fn config_stream(s: &mut TcpStream) -> std::io::Result<()> {
    Ok(())
}
