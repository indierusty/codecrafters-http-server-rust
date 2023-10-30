// Uncomment this block to pass the first stage
use std::io::{Read, Result, Write};
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
    let mut buffer = [0; 2048];
    stream.read(&mut buffer).unwrap();
    let request_str = std::str::from_utf8(&buffer).unwrap();

    let path = request_str.split_whitespace().nth(1).unwrap();
    let random_str = path.split(|c| c == '/').nth(2).unwrap();

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        random_str.len(),
        random_str
    );

    stream.write(response.as_bytes()).unwrap();
}
