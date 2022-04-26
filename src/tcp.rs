use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use crate::http::handle_connection;
use crate::lib::ThreadPool;

pub fn handle_tcp_connection(
    host: String,
    port: String,
    path_arc: Arc<Mutex<String>>,
) -> JoinHandle<()> {
    let listener = TcpListener::bind(format!("{}:{}", host, port)).unwrap();

    let thread_handle = thread::spawn(move || {
        let pool = ThreadPool::new(4);

        for stream in listener.incoming() {
            let stream = stream.unwrap();

            let path_owned = path_arc.lock().unwrap().to_owned();

            pool.execute(|| {
                handle_connection(stream, path_owned);
            });
        }
    });

    thread_handle
}
