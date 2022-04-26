use std::collections::HashMap;
use std::io::prelude::*;
use std::net::TcpStream;

use crate::http_get_file::{handle_request, HttpFileResponse};
use crate::http_headers::parse_headers;
use crate::log::log;

pub fn handle_connection(mut stream: TcpStream, dir_path: String) {
    let http_errors: HashMap<usize, &str> = HashMap::from([
        (200, "OK"),
        (500, "Internal Server Error"),
        (501, "Not Implemented"),
        (400, "Bad Request"),
        (404, "Not Found"),
    ]);

    log(format!(
        "handle_connection_ip {}",
        stream.peer_addr().unwrap()
    ));

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

    let response = format!(
        "HTTP/1.1 {} {}\r\n",
        status_code,
        http_errors.get(&&status_code).unwrap(),
    );

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

    log(format!(
        "{} - HTTP {} {} {} bytes",
        stream.peer_addr().unwrap(),
        status_code,
        http_errors.get(&&status_code).unwrap(),
        bytes_written
    ));
}
