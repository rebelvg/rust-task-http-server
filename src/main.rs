use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::MAIN_SEPARATOR;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod lib;

use lib::ThreadPool;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    let mut port: String = String::from("80");
    let mut path: String = format!(
        "{}{}{}",
        env::current_dir().unwrap().to_str().unwrap(),
        MAIN_SEPARATOR,
        "folder"
    );

    for arg in args {
        let arg_split: Vec<&str> = arg.split("=").collect();

        match arg_split[0] {
            "PORT" => {
                port = String::from(arg_split[1]);
            }
            "PATH" => {
                path = String::from(arg_split[1]);
            }
            _ => {}
        }
    }

    println!("server_running");

    let listener_v4 = TcpListener::bind(format!("{}{}", "0.0.0.0:", port)).unwrap();
    let listener_v6 = TcpListener::bind(format!("{}{}", "[::1]:", port)).unwrap();

    let path_arc = Arc::new(Mutex::new(path));

    let path_arc_v4 = Arc::clone(&path_arc);
    let path_arc_v6 = Arc::clone(&path_arc);

    let handle_v4 = thread::spawn(move || {
        let pool = ThreadPool::new(4);

        for stream in listener_v4.incoming() {
            let stream = stream.unwrap();

            let path_owned = path_arc_v4.lock().unwrap().to_owned();

            pool.execute(|| {
                handle_connection(stream, path_owned);
            });
        }
    });

    let handle_v6 = thread::spawn(move || {
        let pool = ThreadPool::new(4);

        for stream in listener_v6.incoming() {
            let stream = stream.unwrap();

            let path_owned = path_arc_v6.lock().unwrap().to_owned();

            pool.execute(|| {
                handle_connection(stream, path_owned);
            });
        }
    });

    handle_v4.join().unwrap();
    handle_v6.join().unwrap();
}

fn handle_connection(mut stream: TcpStream, path: String) {
    let HTTP_ERRORS: HashMap<usize, &str> = HashMap::from([
        (200, "OK"),
        (500, "Internal Server Error"),
        (501, "Not Implemented"),
        (400, "Bad Request"),
        (404, "Not Found"),
    ]);

    println!("handle_connection ip - {}", stream.peer_addr().unwrap());

    let mut buffer = [0; 1024];

    stream.read(&mut buffer).unwrap();

    let parse_res = parse_headers(buffer);

    let mut status_code = 200;
    let mut http_response: String = String::from("");

    match parse_res {
        Err(text) => {
            status_code = 500;
            http_response = text;
        }
        Ok(http_header) => {
            let request_res = handle_request(http_header, path);

            match request_res {
                Err(text) => match text.as_str() {
                    "bad_method" => {
                        status_code = 501;
                        http_response = text;
                    }
                    "bad_path" => {
                        status_code = 400;
                        http_response = text;
                    }
                    "not_found" => {
                        status_code = 404;
                        http_response = text;
                    }
                    _ => {}
                },
                Ok(content) => {
                    status_code = 200;
                    http_response = content;
                }
            }
        }
    }

    // handle closed connection

    // thread::sleep(Duration::from_secs(60));

    let response = format!(
        "HTTP/1.1 {} {}\r\n\r\n{}",
        status_code,
        HTTP_ERRORS.get(&&status_code).unwrap(),
        http_response
    );

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

struct HttpHeaderStruct {
    protocol: String,
    method: String,
    path: String,
    headers: HashMap<String, String>,
}

fn parse_headers(buffer: [u8; 1024]) -> Result<HttpHeaderStruct, String> {
    let headers_hash = HashMap::new();

    let headers_string = String::from_utf8(buffer.to_vec()).unwrap();

    let headers_vec: Vec<&str> = headers_string.split("\r\n").collect();

    println!("{}", headers_string);

    let http_header_vec: Vec<&str> = headers_vec[0].split(" ").collect();

    if http_header_vec.len() != 3 {
        return Err(format!("bad_http_header"));
    }

    if http_header_vec[2] != "HTTP/1.1" {
        return Err(format!("bad_http_protocol"));
    }

    let http_header = HttpHeaderStruct {
        protocol: String::from(http_header_vec[2]),
        method: String::from(http_header_vec[0]),
        path: String::from(http_header_vec[1]),
        headers: headers_hash,
    };

    Ok(http_header)
}

fn handle_request(http_header: HttpHeaderStruct, path: String) -> Result<String, String> {
    if http_header.method != "GET" {
        return Err(format!("bad_method"));
    }

    if !http_header.path.starts_with("/download/") {
        return Err(format!("bad_path"));
    }

    let path_vec: Vec<&str> = http_header.path.split("/download/").collect();

    if path_vec.len() < 2 {
        return Err(format!("bad_path"));
    }

    println!("{} {}", path, path_vec[1]);

    let file_path = format!("{}{}{}", path, MAIN_SEPARATOR, path_vec[1]);

    let file_content = fs::read_to_string(file_path);

    match file_content {
        Err(_text) => {
            return Err(format!("not_found"));
        }
        Ok(content) => {
            return Ok(content);
        }
    }
}