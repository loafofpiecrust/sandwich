use std::net::{TcpListener, TcpStream};
use std::time::Duration;

const SANDWICH_PORT: u16 = 34222;

pub fn find_peer() -> std::io::Result<TcpStream> {
    // TODO templatize this string!
    let host = "sandwich2";
    let url = format!("{}.local:{}", host, SANDWICH_PORT);
    let mut stream = TcpStream::connect(url)?;
    config_stream(&mut stream)?;
    Ok(stream)
}

pub fn wait_for_peer() -> std::io::Result<TcpStream> {
    let mut conn = TcpListener::bind(format!("0.0.0.0:{}", SANDWICH_PORT))?;
    let (mut stream, addr) = conn.accept()?;
    println!("Client connected!!");
    config_stream(&mut stream)?;
    Ok(stream)
}

fn config_stream(s: &mut TcpStream) -> std::io::Result<()> {
    s.set_write_timeout(Some(Duration::from_secs(5)))?;
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    Ok(())
}
