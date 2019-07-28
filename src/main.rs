use threadpool::ThreadPool;
use mysql::params;

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::collections::HashMap;

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
    let mut buffer: Vec<u8> = Vec::new();
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
            buffer.push(*x);
        }
    }
    let request = buffer.iter().map(|x| {*x as char}).collect::<String>();
    let mut response = "HTTP/1.1 405 Method Not Allowed\r\n\r\nThe OLMMCC api only supports multipart/form-data.".to_string();
    if request.contains("multipart/form-data") {
        let url = request.split_ascii_whitespace().collect::<Vec<&str>>()[1];
        let body = request.split("\r\n\r\n").skip(1).collect::<Vec<&str>>();
        response = formulate_response(url, get_form_data(body));
    }
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
fn get_form_data(body: Vec<&str>) -> HashMap<&str, &str> {
    let mut hash_map = HashMap::new();
    for i in (0..(body.len() / 2)).map(|x| {x * 2}) {
        hash_map.insert(
            body[i].split("\"").collect::<Vec<&str>>()[1], 
            body[i+1].split("\r\n").collect::<Vec<&str>>()[0],
        );
    }
    hash_map
}
fn formulate_response(url: &str, body: HashMap<&str, &str>) -> String {
    match url {
        "/get_page" => {
            let mut builder = mysql::OptsBuilder::new();
            builder
                .db_name(Some("olmmcc"))
                .user(Some("justus"))
                .pass(Some(""));
            let mut pool = mysql::Conn::new(builder).unwrap();
            let result: Vec<String> = pool
                .prep_exec(
                    "SELECT * FROM pages where topnav_id=:a", 
                    params!("a" => body.get("page").unwrap())
                )
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