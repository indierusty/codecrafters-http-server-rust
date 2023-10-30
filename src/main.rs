use std::collections::HashMap;
// Uncomment this block to pass the first stage
use std::io::{Read, Result, Write};
use std::net::{TcpListener, TcpStream};
use threadpool::ThreadPool;

const ADDRESS: &str = "127.0.0.1:4221";

fn main() -> Result<()> {
    let listener = TcpListener::bind(ADDRESS)?;

    let pool = ThreadPool::new(4);
    for stream in listener.incoming() {
        pool.execute(|| handle_client(stream.unwrap()));
    }

    Ok(())
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 2048];
    stream.read(&mut buffer).unwrap();
    let request_str = std::str::from_utf8(&buffer).unwrap();

    let path = request_str.split_whitespace().nth(1).unwrap();

    let response = match path {
        "/" => "HTTP/1.1 200 OK \r\n\r\n".to_string(),
        c if c.starts_with("/echo") => {
            let random_str = path
                .trim_matches('/')
                .split_once(|c| c == '/')
                .unwrap()
                .1
                .to_string();

            format!(
                "HTTP/1.1 200 OK\r\nContent-type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                random_str.len(),
                random_str
            )
        }
        "/user-agent" => {
            // parse headers
            let mut headers: HashMap<&str, &str> = HashMap::new();
            for line in request_str.lines() {
                if let Some((k, v)) = line.split_once(":") {
                    headers.insert(k, v);
                }
            }

            let user_agent = headers
                .get("User-Agent")
                .unwrap_or(&"NOTFOUND user-agent")
                .trim();

            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}\r\n",
                user_agent.len(),
                user_agent
            )
        }
        _ => "HTTP/1.1 404 NOT FOUND\r\n\r\n".to_string(),
    };

    stream.write(response.as_bytes()).unwrap();
}
