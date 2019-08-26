use scrypt::{scrypt_check, scrypt_simple, ScryptParams};
use serde_json::json;

use std::collections::HashMap;
use std::io::prelude::*;
use std::net::TcpStream;

mod request_functions;
use request_functions::*;

const BUFFER_SIZE: usize = 128;

pub fn handle_connection(mut stream: TcpStream) {
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
    let request = buffer.iter().map(|x| *x as char).collect::<String>();
    let mut response =
        "HTTP/1.1 405 Method Not Allowed\r\n\r\nThe OLMMCC api only supports multipart/form-data."
            .to_string();
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
    for i in 0..(body.len() - 1) {
        hash_map.insert(
            body[i].split("name=\"").collect::<Vec<&str>>()[1]
                .split("\"")
                .next()
                .unwrap(),
            body[i + 1].split("\r\n").collect::<Vec<&str>>()[0],
        );
    }
    hash_map
}
fn ok(body: &str) -> String {
    format!("HTTP/1.1 200 Ok\r\n\r\n{}", body)
}
fn j_ok(body: serde_json::Value) -> String {
    ok(&body.to_string())
}
fn formulate_response(url: &str, body: HashMap<&str, &str>) -> String {
    match url {
        "/get_page" => get_page(body),
        "/get_songs" => get_songs(),
        "/get_image_list" => get_image_list(),
        "/get_calendar_events" => get_calendar_events(body),
        "/signup" => signup(body),
        "/login" => login(body),
        "/kill_session" => kill_session(body),
        "/get_account" => get_account(body),
        "/change_password" => change_password(body),
        "/refresh" => refresh(body),
        "/change_username" => change_username(body),
        "/change_subscription" => change_subscription(body),
        "/change_email" => change_email(body),
        "/delete_account" => delete_account(body),
        "/get_database" => get_database(body),
        "/get_row_titles" => get_row_titles(body),
        _ => format!(
            "HTTP/1.1 404 Not Found\r\n\r\nThe provided url {} could not be resolved.",
            url
        ),
    }
}
fn message(message: &str) -> String {
    ok(&json!({ "message": message }).to_string())
}
fn hash(to_hash: &str) -> String {
    scrypt_simple(to_hash, &ScryptParams::new(12, 8, 1).unwrap()).unwrap()
}
fn hash_match(password: &str, hash: &str) -> bool {
    scrypt_check(password, hash).is_ok()
}
