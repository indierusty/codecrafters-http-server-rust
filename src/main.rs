use std::collections::HashMap;
use std::fs;
// Uncomment this block to pass the first stage
use std::io::{Read, Result, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
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
        c if c.starts_with("/files") => {
            let file_name = Path::new(path.split_once("files/").unwrap().1);

            let mut file_path = "".to_string();
            let mut arg = std::env::args().peekable();

            loop {
                if let Some(next_arg) = arg.next() {
                    if next_arg == "--directory" {
                        if let Some(file) = arg.next() {
                            file_path = file.to_string();
                        }
                    }
                } else {
                    break;
                }
            }

            let path = Path::new(&file_path).join(Path::new(file_name));
            if path.is_file() {
                let file_content = fs::read_to_string(path).unwrap();
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}\r\n",
                    file_content.len(),
                    file_content
                )
            } else {
                "HTTP/1.1 404 NOT FOUND\r\n\r\n".to_string()
            }
        }
        _ => "HTTP/1.1 404 NOT FOUND\r\n\r\n".to_string(),
    };

    stream.write(response.as_bytes()).unwrap();
}
