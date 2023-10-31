use nom::AsBytes;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Result, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use threadpool::ThreadPool;

const ADDRESS: &str = "127.0.0.1:4221";

fn main() -> Result<()> {
    let listener = TcpListener::bind(ADDRESS)?;

    let pool = ThreadPool::new(4);
    for stream in listener.incoming() {
        // pool.execute(|| handle_client(stream.unwrap()));
        pool.execute(|| handle(stream.unwrap()));
    }

    Ok(())
}

#[derive(Debug)]
enum Method {
    GET,
    POST,
}

#[derive(Debug)]
struct Request {
    method: Method,
    path: String,
    http_version: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Request {
    fn new() -> Self {
        Self {
            method: Method::GET,
            path: "".to_string(),
            http_version: "".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    fn parse(stream: &mut TcpStream) -> Self {
        const BUF_LEN: usize = 600;
        let mut buf = [0; BUF_LEN];
        stream.read(&mut buf).expect("read steam to end");

        let read_len = buf
            .iter()
            .enumerate()
            .find(|(_, b)| **b == 0)
            .unwrap_or((BUF_LEN, &0))
            .0;

        // DEGUB:
        println!("read len {}", read_len);

        // NOTE: any u8 value is valid aschii char hence
        // i can get raw bytes after i convert buf(u8[]) -> string(utf-8) type
        let buf = String::from_utf8(buf.to_vec()).unwrap();
        let mut req = Request::new();

        let mut headers_ended = false;

        for (i, line) in buf.lines().enumerate() {
            if i == 0 {
                let l = line.split_whitespace().collect::<Vec<&str>>();
                req.method = match l[0] {
                    l if l.starts_with("GET") => Method::GET,
                    l if l.starts_with("POST") => Method::POST,
                    _ => Method::GET,
                };

                req.path.push_str(l[1]);

                req.http_version.push_str(l[2]);

                continue;
            }

            // Headers
            if let Some((k, v)) = line.split_once(":") {
                req.headers
                    .insert(k.trim().to_string(), v.trim().to_string());
            } else {
                headers_ended = true;
            }

            // Body
            if headers_ended {
                req.body.append(&mut line.as_bytes().to_vec())
            }
        }

        // count number of bytes already read and how much to read more
        // then read rest of request and then parse body
        if let Some(body_size) = req.headers.get("Content-Length") {
            let body_size: usize = body_size.parse().unwrap();

            // NOTE: make sure first buf reading is reads enough to include
            // '\r\n\r\n' whitspace at end of headers in request

            if let Some(mut end_header_index) = buf.find("\r\n\r\n") {
                end_header_index += "\r\n\r\n".len();

                let full_request_size = body_size + end_header_index;

                let rest_size = if full_request_size - read_len > 0 {
                    full_request_size - read_len
                } else {
                    0
                };

                // DEBUG:
                println!("full: {} rest {}", full_request_size, rest_size);

                let mut rest_buf: Vec<u8> = Vec::new();
                if rest_size > 0 {
                    rest_buf.resize(rest_size, 0);

                    stream.read(&mut rest_buf[..rest_size]).unwrap();
                }

                let full_request = [buf.as_bytes(), &rest_buf].concat();
                let body = full_request.split_at(end_header_index).1;

                req.body = body[..body_size].to_vec();
                // DEBUG:
                println!("{:?}", String::from_utf8(req.body.to_vec()));
                println!("{:?}", String::from_utf8(body.to_vec()));
            }
        }

        req
    }
}

struct Response {
    http_version: String,
    status_code: usize,
    status_message: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Response {
    fn new() -> Self {
        Self {
            http_version: String::new(),
            status_code: 0,
            status_message: String::new(),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    fn http_version(mut self, version: String) -> Self {
        self.http_version = version;
        self
    }

    fn status_code(mut self, code: usize) -> Self {
        self.status_code = code;
        self
    }

    fn status_message(mut self, message: String) -> Self {
        self.status_message = message;
        self
    }

    fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    fn not_found() -> Self {
        Self::new()
            .http_version("HTTP/1.1".to_string())
            .status_code(404)
            .status_message("NOT FOUND".to_string())
    }

    fn ok() -> Self {
        Self::new()
            .http_version("HTTP/1.1".to_string())
            .status_code(200)
            .status_message("OK".to_string())
    }

    fn write(&self, stream: &mut TcpStream) {
        // DEBUG:
        println!("WRITE");
        let mut res = format!(
            "{} {} {}\r\n",
            self.http_version, self.status_code, self.status_message
        );

        for (k, v) in &self.headers {
            res.push_str(&k);
            res.push_str(": ");
            res.push_str(&v);
            res.push_str("\r\n");
        }

        res.push_str("\r\n"); // end of headers

        // include BODY and write to stream
        stream
            .write(&[res.as_bytes(), &self.body[..]].concat())
            .unwrap();

        // DEBUG:
        print!("{res}");
    }
}

fn echo(request: &Request) -> Response {
    let random_string = request
        .path
        .trim_matches('/')
        .split_once(|c| c == '/')
        .unwrap()
        .1
        .to_string();

    let mut headers = HashMap::new();
    headers.insert("Content-type".to_string(), "text/plain".to_string());
    headers.insert(
        "Content-length".to_string(),
        format!("{}", random_string.len()),
    );

    Response::new()
        .http_version("HTTP/1.1".to_string())
        .status_code(200)
        .status_message("OK".to_string())
        .headers(headers)
        .body(random_string.as_bytes().to_vec())
}

fn user_agent(request: &Request) -> Response {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "text/plain".to_string());
    headers.insert(
        "Content-Length".to_string(),
        format!("{}", request.headers.get("User-Agent").unwrap().len()),
    );

    Response::new()
        .http_version("HTTP/1.1".to_string())
        .status_code(200)
        .status_message("OK".to_string())
        .body(
            request
                .headers
                .get("User-Agent")
                .unwrap()
                .as_bytes()
                .to_vec(),
        )
}

fn get_file(request: &Request) -> Response {
    let file_name = Path::new(request.path.split_once("files/").unwrap().1);

    let mut dir_path = "".to_string();
    let mut arg = std::env::args().peekable();

    loop {
        if let Some(next_arg) = arg.next() {
            if next_arg == "--directory" {
                if let Some(file) = arg.next() {
                    dir_path = file.to_string();
                }
            }
        } else {
            break;
        }
    }

    let path = Path::new(&dir_path).join(Path::new(file_name));
    if path.is_file() {
        let file_content = fs::read_to_string(path).unwrap();
        // format!(
        //             "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}\r\n",
        //             file_content.len(),
        //             file_content
        //         )
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            "application/octent-stream".to_string(),
        );
        headers.insert(
            "Content-Length".to_string(),
            format!("{}", file_content.len()),
        );

        Response::new()
            .http_version("HTTP/1.1".to_string())
            .status_code(200)
            .status_message("OK".to_string())
            .headers(headers)
            .body(file_content.as_bytes().to_vec())
    } else {
        // "HTTP/1.1 404 NOT FOUND\r\n\r\n".to_string()
        Response::not_found()
    }
}

fn post_file(request: &Request) -> Response {
    let file_name = Path::new(request.path.split_once("files/").unwrap().1);

    let mut dir_path = "".to_string();
    let mut arg = std::env::args().peekable();

    loop {
        if let Some(next_arg) = arg.next() {
            if next_arg == "--directory" {
                if let Some(file) = arg.next() {
                    dir_path = file.to_string();
                }
            }
        } else {
            break;
        }
    }

    // println!("{:?}", request.body);

    let path = Path::new(&dir_path).join(Path::new(file_name));
    let mut file = fs::File::create(path).unwrap();

    file.write(&request.body)
        .expect("Write request body to file");

    Response::new()
}

fn handle(mut stream: TcpStream) {
    let request = Request::parse(&mut stream);

    let response = match request.method {
        Method::GET => match request.path.as_str() {
            "/" => Response::ok(),
            "/user-agent" => user_agent(&request),
            r if r.starts_with("/echo") => echo(&request),
            r if r.starts_with("/files") => get_file(&request),
            _ => Response::not_found(),
        },
        Method::POST => match request.path.as_str() {
            r if r.starts_with("/files") => post_file(&request),
            _ => Response::not_found(),
        },
    };

    response.write(&mut stream);
    return;
}
