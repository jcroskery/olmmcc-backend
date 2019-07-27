use threadpool::ThreadPool;

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use mysql::params;

const BUFFER_SIZE: usize = 128;
const NUM_THREADS: usize = 4;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:3000").unwrap();
    let pool = ThreadPool::new(NUM_THREADS);
    for stream in listener.incoming() {
        pool.execute(move || {
            handle_connection(stream.unwrap());
        });
    }
}
fn handle_connection(mut stream: TcpStream) {
    let mut buffer: Vec<char> = Vec::new();
    let mut eof = false;
    while !eof {
        let mut peek_buf = [0; BUFFER_SIZE + 1];
        stream.peek(&mut peek_buf).unwrap();
        if peek_buf[BUFFER_SIZE] == 0 {
            eof = true;
        }

        let mut read_buf = [0; BUFFER_SIZE];
        stream.read(&mut read_buf).unwrap();
        for x in read_buf.into_iter() {
            if *x == 0 {
                eof = true;
                break;
            }
            buffer.push(*x as char);
        }
    }
    let request: String = buffer.iter().collect();
    println!("{}", request);
    let mut response = "HTTP/1.1 405 Method Not Allowed\r\n\r\nThe OLMMCC api only supports POST.".to_string();
    if request.contains("POST") {
        let mut split_at_post = request.split("POST ");
        split_at_post.next();
        let url = split_at_post.next().unwrap()
            .split_ascii_whitespace().next().unwrap();
        let mut split_header_body = request.split("\r\n\r\n");
        split_header_body.next();
        let body = split_header_body.next().unwrap();
        response = formulate_response(url, body);
    }
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
fn formulate_response(url: &str, body: &str) -> String {
    match url {
        "/get_page" => {
            let mut builder = mysql::OptsBuilder::new();
            builder.db_name(Some("olmmcc")).user(Some("justus")).pass(Some(""));
            let mut pool = mysql::Conn::new(builder).unwrap();
            let result: Vec<String> = pool
                .prep_exec("SELECT * FROM pages where topnav_id=:a", params!("a" => "home"))
                .unwrap()
                .map(|row| {
                    let (_, text, _) = mysql::from_row::
                        <(i32, String, String)>(row.unwrap());
                    htmlescape::decode_html(&text).unwrap()
                })
                .collect(); 
            format!("HTTP/1.1 200 Ok\r\n\r\n{}", result[0].to_string())
        }
        _ => {
            "HTTP/1.1 404 Not Found\r\n\r\nUrl could not be resolved.".to_string()
        }
    }
}