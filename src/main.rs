use threadpool::ThreadPool;
use core::convert::TryFrom;

use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use mysql::params;
use http_header::RequestHeader;

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
    let header = http_header::Header::parse(&buffer).unwrap();
    let parsed_request = RequestHeader::try_from(header).unwrap();
    let mut response = "HTTP/1.1 405 Method Not Allowed\r\n\r\nThe OLMMCC api only supports POST.".to_string();
    if parsed_request.method().iter().map(|x| {*x as char}).collect::<String>() == "POST" {
        let url = parsed_request.uri().iter().map(|x| {*x as char}).collect::<String>();
        println!("{}", parsed_request.fields().iter().map(|x| {
            x.1.iter().map(|x| {*x as char}).collect::<String>()
        }).collect::<String>());
        //let mut split_header_body = request.split("\r\n\r\n");
        //split_header_body.next();
        //let body = split_header_body.next().unwrap();
        //response = formulate_response(url, body);
    }
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
fn formulate_response(url: &str, body: &str) -> String {
    match url {
        "/get_page" => {
            let body_sep: Vec<&str> = body.split("=").collect();
            let mut builder = mysql::OptsBuilder::new();
            builder.db_name(Some("olmmcc")).user(Some("justus")).pass(Some(""));
            let mut pool = mysql::Conn::new(builder).unwrap();
            let result: Vec<String> = pool
                .prep_exec("SELECT * FROM pages where topnav_id=:a", params!("a" => "home"))
                .unwrap()
                .map(|row| {
                    let (_, text, _) = mysql::from_row::
                        <(i32, String, String)>(row.unwrap());
                    //htmlescape::decode_html(&text).unwrap()
                    body.to_string()
                })
                .collect(); 
            format!("HTTP/1.1 200 Ok\r\n\r\n{}", result[0].to_string())
        }
        _ => {
            "HTTP/1.1 404 Not Found\r\n\r\nUrl could not be resolved.".to_string()
        }
    }
}