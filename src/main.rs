use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::MAIN_SEPARATOR;
use std::path::{Component, Path};
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

fn handle_connection(mut stream: TcpStream, dir_path: String) {
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

    let mut status_code = 200;
    let mut http_response: String = String::from("");
    let mut http_file: Option<HttpFileResponse> = None;
    let mut is_chunked = false;

    match parse_headers(buffer) {
        Err(text) => {
            status_code = 500;
            http_response = text;
        }
        Ok(http_header) => match handle_request(&http_header, dir_path) {
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
                http_file = Some(content);

                if http_header.headers.get("Transfer-Encoding") == Some(&String::from("chunked")) {
                    is_chunked = true;
                }
            }
        },
    }

    // handle closed connection

    // thread::sleep(Duration::from_secs(60));

    let response = format!(
        "HTTP/1.1 {} {}\r\n",
        status_code,
        HTTP_ERRORS.get(&&status_code).unwrap(),
    );

    println!("response - {}", response);

    stream.write(response.as_bytes()).unwrap();

    let mut bytes_written = 0;

    if let Some(mut value) = http_file {
        if !is_chunked {
            stream
                .write(format!("{}{}", "Content-Length: ", value.size(),).as_bytes())
                .unwrap();
        } else {
            stream
                .write(format!("Transfer-Encoding: chunked",).as_bytes())
                .unwrap();
        }

        stream.write(format!("\r\n\r\n",).as_bytes()).unwrap();

        let mut buffer = [0; 1024 * 256];

        loop {
            match value.file.read(&mut buffer) {
                Ok(bytes) => {
                    bytes_written += bytes;

                    if bytes != 0 {
                        if !is_chunked {
                            stream.write(&buffer[..(bytes)]).unwrap();
                        } else {
                            stream
                                .write(
                                    format!(
                                        "{}\r\n{}\r\n",
                                        &buffer[..(bytes)].len(),
                                        String::from_utf8(buffer[..(bytes)].to_vec()).unwrap()
                                    )
                                    .as_bytes(),
                                )
                                .unwrap();
                        }
                    } else {
                        break;
                    }
                }
                _ => {
                    break;
                }
            }
        }

        if is_chunked {
            stream.write(format!("{}\r\n\r\n", 0,).as_bytes()).unwrap();
        }
    } else {
        println!("http_response - {}", http_response);

        stream
            .write(
                format!(
                    "{}{}\r\n\r\n{}",
                    "Content-Length: ",
                    http_response.len(),
                    http_response
                )
                .as_bytes(),
            )
            .unwrap();
    }

    stream.write(format!("\r\n",).as_bytes()).unwrap();

    stream.flush().unwrap();

    println!("bytes_written - {}", bytes_written);
}

struct HttpHeaderStruct {
    protocol: String,
    method: String,
    path: String,
    headers: HashMap<String, String>,
}

struct HttpFileResponse {
    file: File,
}

impl HttpFileResponse {
    fn new(file: File) -> HttpFileResponse {
        HttpFileResponse { file }
    }

    fn size(&self) -> usize {
        self.file.metadata().unwrap().len() as usize
    }
}

fn parse_headers(buffer: [u8; 1024]) -> Result<HttpHeaderStruct, String> {
    let mut headers_hash = HashMap::new();

    let headers_string = String::from_utf8(buffer.to_vec()).unwrap();

    let headers_vec: Vec<&str> = headers_string.split("\r\n").collect();

    let http_header_vec: Vec<&str> = headers_vec[0].split(" ").collect();

    if http_header_vec.len() != 3 {
        return Err(format!("bad_http_header"));
    }

    if http_header_vec[2] != "HTTP/1.1" {
        return Err(format!("bad_http_protocol"));
    }

    for header_line in headers_vec.iter() {
        let header_line_vec: Vec<&str> = header_line.split(": ").collect();

        if header_line_vec.len() == 2 {
            headers_hash.insert(
                String::from(header_line_vec[0]),
                String::from(header_line_vec[1]),
            );
        }
    }

    let http_header = HttpHeaderStruct {
        protocol: String::from(http_header_vec[2]),
        method: String::from(http_header_vec[0]),
        path: String::from(http_header_vec[1]),
        headers: headers_hash,
    };

    Ok(http_header)
}

fn handle_request(
    http_header: &HttpHeaderStruct,
    dir_path: String,
) -> Result<HttpFileResponse, String> {
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

    println!("{} {}", dir_path, path_vec[1]);

    let file_path = Path::new(&dir_path);

    let p = Path::new(&path_vec[1]);

    if p.components()
        .into_iter()
        .any(|x| x == Component::ParentDir)
    {
        return Err(format!("bad_path"));
    }

    match File::open(file_path.join(path_vec[1])) {
        Err(_text) => {
            return Err(format!("not_found"));
        }
        Ok(content) => {
            return Ok(HttpFileResponse::new(content));
        }
    }
}
