use std::fs::File;
use std::path::{Component, Path};

use crate::http_headers::HttpHeaderStruct;

pub struct HttpFileResponse {
    pub file: File,
}

impl HttpFileResponse {
    fn new(file: File) -> HttpFileResponse {
        HttpFileResponse { file }
    }

    pub fn size(&self) -> usize {
        self.file.metadata().unwrap().len() as usize
    }
}

pub fn handle_request(
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
