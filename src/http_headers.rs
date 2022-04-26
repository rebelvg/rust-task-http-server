use std::collections::HashMap;

pub struct HttpHeaderStruct {
    pub protocol: String,
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
}

pub fn parse_headers(buffer: [u8; 1024]) -> Result<HttpHeaderStruct, String> {
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
