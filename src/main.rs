// Uncomment this block to pass the first stage
use anyhow::Result;
use std::io::Write;
use std::net::{TcpListener, TcpStream};

const ADDRESS: &str = "127.0.0.1:4221";

fn main() -> Result<()> {
    let listener = TcpListener::bind(ADDRESS)?;

    for stream in listener.incoming() {
        handle_client(stream?)
    }

    Ok(())
}

fn handle_client(mut stream: TcpStream) {
    let response = "HTTP/1.1 200 0K\r\n\r\n";
    stream.write(response.as_bytes()).unwrap();
}
