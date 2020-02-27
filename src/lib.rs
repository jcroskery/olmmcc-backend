use scrypt::{scrypt_check, scrypt_simple, ScryptParams};
use serde_json::json;

use std::collections::HashMap;

mod request_functions;
use request_functions::*;

/*
async fn handle_request(request: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());

    match request.method() {
        &Method::POST => {
            let full_body = hyper::body::to_bytes(request.into_body()).await?;
            let 
        },
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        },
    }
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
        let regex = Regex::new(r"(\\r\\n\\r\\n)").unwrap();
        let unsplit_body = regex.replace(&request, "");
        let other_regex = Regex::new("\"\\r\\n\\r\\n").unwrap();
        let body = other_regex.split(&unsplit_body).collect();
        response = formulate_response(url, get_form_data(body));
    }
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
*/
fn ok(body: &str) -> String {
    body.to_string()
}
fn j_ok(body: serde_json::Value) -> String {
    body.to_string()
}
pub fn formulate_response(url: &str, body: HashMap<&str, &str>) -> String {
    match url {
        "/get_songs" => get_songs(),
        "/hash_password" => hash_password(body),
        "/get_image_list" => get_image_list(),
        "/get_calendar_events" => get_calendar_events(body),
        "/signup" => signup(body),
        "/login" => login(body),
        "/admin_login" => admin_login(body),
        "/kill_session" => kill_session(body),
        "/get_account" => get_account(body),
        "/refresh" => refresh(body),
        "/change_subscription" => change_subscription(body),
        "/send_change_email" => send_change_email(body),
        "/send_delete_email" => send_delete_email(body),
        "/change_email" => change_email(body),
        "/delete_account" => delete_account(body),
        "/get_database" => get_database(body),
        "/get_row_titles" => get_row_titles(body),
        "/move_row_to_end" => move_row_to_end(body),
        "/move_row_to_start" => move_row_to_start(body),
        "/delete_row" => delete_row(body),
        "/add_row" => add_row(body),
        "/change_row" => change_row(body),
        "/get_gmail_auth_url" => get_gmail_auth_url(body),
        "/is_gmail_working" => is_gmail_working(body),
        "/send_gmail_code" => send_gmail_code(body),
        "/verify_account" => verify_account(body),
        "/send_email" => send_email(body),
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
