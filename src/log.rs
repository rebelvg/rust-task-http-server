use std::fs::OpenOptions;
use std::io::prelude::*;
use std::time::SystemTime;

pub fn log(log_line: String) {
    println!("log {}", log_line);

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("logs.txt")
        .unwrap();

    if let Err(e) = writeln!(file, "{:?}: {}", SystemTime::now(), log_line) {
        eprintln!("could not write to file {}", e);
    }
}
