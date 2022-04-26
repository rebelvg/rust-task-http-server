use std::env;
use std::path::MAIN_SEPARATOR;
use std::sync::{Arc, Mutex};

mod http;
mod http_get_file;
mod http_headers;
mod lib;
mod tcp;

use crate::tcp::handle_tcp_connection;

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

    let path_arc = Arc::new(Mutex::new(path));

    let path_arc_v4 = Arc::clone(&path_arc);
    let path_arc_v6 = Arc::clone(&path_arc);

    let handle_v4 = handle_tcp_connection("0.0.0.0".to_string(), port.clone(), path_arc_v4);
    let handle_v6 = handle_tcp_connection("[::1]".to_string(), port.clone(), path_arc_v6);

    println!("server_running");

    handle_v4.join().unwrap();
    handle_v6.join().unwrap();
}
