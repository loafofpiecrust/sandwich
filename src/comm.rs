use std::net::{TcpListener, TcpStream};
use std::time::Duration;

const SERVICE_NAME: &str = "_printer._tcp";
const SANDWICH_PORT: u16 = 34222;

pub fn find_peer() -> std::io::Result<TcpStream> {
    // TODO templatize this string!
    let url = "sandwich2.local:34222";
    let mut stream = TcpStream::connect(url)?;
    config_stream(&mut stream)?;
    Ok(stream)
}

pub fn wait_for_peer() -> std::io::Result<TcpStream> {
    let mut conn = TcpListener::bind("0.0.0.0:34222")?;
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
