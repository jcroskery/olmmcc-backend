use chrono::NaiveDate;
use htmlescape::decode_html;
use mysql::from_value;
use serde::Serialize;
use serde_json::{json, Map, Value};
use session::Session;

use std::collections::HashMap;
use std::fs;
use std::io::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

mod account_validation;
use account_validation::*;

mod database_functions;
use database_functions::*;

use crate::{hash, hash_match, j_ok, message, ok};

#[derive(Serialize)]
struct Song {
    name: String,
    link: String,
    role: String,
}

#[derive(Serialize)]
struct SongArticle {
    title: String,
    text: String,
    songs: Vec<Song>,
}

#[derive(Serialize)]
struct CalendarEvent {
    id: i64,
    title: String,
    date: String,
    start_time: String,
    end_time: String,
    notes: String,
}

pub fn get_page(body: HashMap<&str, &str>) -> String {
    ok(&decode_html(&from_value::<String>(
        get_like("pages", "topnav_id", body["page"])[0][1].clone(),
    ))
    .unwrap())
}
pub fn get_songs() -> String {
    let mut expiry = 0;
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut article: Vec<SongArticle> = get_all_rows("articles")
        .iter()
        .filter(|row| {
            let this_expiry = from_value::<NaiveDate>(row[3].clone())
                .and_hms(0, 0, 0)
                .timestamp();
            if this_expiry > expiry && current_time < this_expiry as u64 {
                expiry = this_expiry;
                true
            } else {
                false
            }
        })
        .map(|x| SongArticle {
            title: from_value(x[1].clone()),
            text: from_value(x[2].clone()),
            songs: Vec::new(),
        })
        .collect();
    match article.pop() {
        Some(mut t) => {
            t.songs = get_like("songs", "article", &t.title)
                .into_iter()
                .map(|x| Song {
                    name: from_value(x[1].clone()),
                    link: from_value(x[2].clone()),
                    role: from_value(x[3].clone()),
                })
                .collect();
            ok(&serde_json::to_string(&t).unwrap())
        }
        None => j_ok(json!({"title" : ""})),
    }
}

pub fn get_image_list() -> String {
    let paths: Vec<String> = fs::read_dir("/srv/http/images/")
        .unwrap()
        .map(|x| x.unwrap().file_name().into_string().unwrap())
        .collect();
    j_ok(json!({ "images": paths }))
}

pub fn get_calendar_events(body: HashMap<&str, &str>) -> String {
    let result: Vec<CalendarEvent> = get_like("calendar", "date", body["year_month"])
        .iter()
        .map(|x| CalendarEvent {
            id: from_value(x[0].clone()),
            title: from_value(x[1].clone()),
            date: from_value::<NaiveDate>(x[2].clone())
                .format("%Y-%m-%d")
                .to_string(),
            start_time: from_value(x[3].clone()),
            end_time: from_value(x[4].clone()),
            notes: from_value(x[5].clone()),
        })
        .collect();
    ok(&serde_json::to_string(&result).unwrap())
}

pub fn signup(body: HashMap<&str, &str>) -> String {
    let email = body["email"].to_lowercase();
    let username = body["username"];
    let password_one = body["password1"];
    let password_two = body["password2"];
    if let Some(t) = check_passwords(password_one, password_two) {
        return message(t);
    }
    if let Some(t) = check_email(&email) {
        return message(t);
    }
    if let Some(t) = check_username(username) {
        return message(t);
    }
    insert_row(
        "users",
        vec![
            "email",
            "username",
            "password",
            "verified",
            "admin",
            "subscription_policy",
            "invalid_email",
        ],
        vec![&email, username, &hash(password_one), "0", "0", "1", "0"],
    )
    .unwrap();
    j_ok(json!({"url" : "/login"}))
}

pub fn login(body: HashMap<&str, &str>) -> String {
    let email = body["email"].to_lowercase();
    let mut session = Session::new(30, 100);
    if let Some(message) = refresh_session(&mut session, "email", email, Some(body["password"])) {
        j_ok(json!({"url" : "", "message" : message}))
    } else {
        j_ok(
            json!({"url" : "/", "session" : session.get_id(), "message" : "Successfully logged in!"}),
        )
    }
}

fn refresh_session(
    session: &mut Session,
    key: &str,
    value: String,
    password: Option<&str>,
) -> Option<String> {
    let users = get_like("users", key, &value);
    if let Some(user) = users.iter().next() {
        if let Some(p) = password {
            if !hash_match(p, &from_value::<String>(user[2].clone())) {
                return Some("Wrong password, please try again.".to_string());
            }
        }
        if from_value::<i32>(user[4].clone()) == 1 {
            session
                .set("id", from_value::<i32>(user[3].clone()).to_string())
                .set("verified", "1".to_string())
                .set(
                    "invalid_email",
                    from_value::<i32>(user[7].clone()).to_string(),
                )
                .set("email", from_value(user[0].clone()))
                .set("username", from_value(user[1].clone()))
                .set("admin", from_value::<i32>(user[5].clone()).to_string())
                .set(
                    "subscription_policy",
                    from_value::<i32>(user[6].clone()).to_string(),
                );
            None
        } else {
            Some("This account has not been verified.".to_string())
        }
    } else {
        Some(format!("Wrong {}, please try again.", key))
    }
}

pub fn get_account(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("verified").unwrap() == "1" {
            const ALLOWED_VARS: &[&str] = &["email", "username", "admin", "subscription_policy"];
            let mut map = Map::new();
            for var in ALLOWED_VARS {
                if body["details"].contains(var) {
                    map.insert(var.to_string(), Value::String(session.get(var).unwrap()));
                }
            }
            return ok(&serde_json::to_string(&map).unwrap());
        }
    }
    j_ok(json!({"session" : "none"}))
}

pub fn kill_session(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        session.delete();
    }
    ok("")
}

pub fn change_password(body: HashMap<&str, &str>) -> String {
    let mut session = Session::from_id(body["session"]).unwrap();
    let password_hash = get_like("users", "email", &session.get("email").unwrap())
        .iter()
        .next()
        .unwrap()[2]
        .clone();
    if hash_match(
        body["current_password"],
        &from_value::<String>(password_hash),
    ) {
        let password_one = body["password1"];
        let password_two = body["password2"];
        if let Some(t) = check_passwords(password_one, password_two) {
            return message(t);
        }
        change_row_where(
            "users",
            "id",
            &session.get("id").unwrap(),
            "password",
            &hash(password_one),
        );
        j_ok(json!({"message" : "Your password was successfully changed!"}))
    } else {
        j_ok(json!({"message" : "Wrong password, please try again."}))
    }
}

pub fn refresh(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        let id = session.get("id").unwrap();
        refresh_session(&mut session, "id", id, None);
    }
    ok("")
}

pub fn change_username(body: HashMap<&str, &str>) -> String {
    let mut session = Session::from_id(body["session"]).unwrap();
    if let Some(t) = check_username(body["username"]) {
        return message(t);
    }
    change_row_where(
        "users",
        "id",
        &session.get("id").unwrap(),
        "username",
        body["username"],
    );
    j_ok(json!({"message" : "Your username was successfully changed!"}))
}

pub fn change_subscription(body: HashMap<&str, &str>) -> String {
    const SUBSCRIPTION_MESSAGES: &[&str] = &[
        "You are now unsubscribed from receiving emails.",
        "You are now subscribed to receive emails.",
        "You are now subscribed to receive emails and reminders.",
    ];
    let mut session = Session::from_id(body["session"]).unwrap();
    if let Some(t) = check_subscription(body["subscription"]) {
        return message(t);
    }
    change_row_where(
        "users",
        "id",
        &session.get("id").unwrap(),
        "subscription_policy",
        body["subscription"],
    );
    j_ok(json!({
        "message" : SUBSCRIPTION_MESSAGES[body["subscription"].parse::<usize>().unwrap()]
    }))
}

pub fn change_email(body: HashMap<&str, &str>) -> String {
    // An email needs to be added to the queue here
    let mut session = Session::from_id(body["session"]).unwrap();
    if session.get("verified").unwrap() == "1" {
        if let Some(t) = check_email(body["email"]) {
            return message(t);
        }
        let message = format!("An email containing an link to change your account email has been sent to {}. Please check your inbox, including the spam folder, for the link. It may take a few minutes to receive the email.", session.get("email").unwrap());
        j_ok(json!({ "message": message }))
    } else {
        ok("")
    }
}

pub fn delete_account(body: HashMap<&str, &str>) -> String {
    // An email needs to be added to the queue here
    let mut session = Session::from_id(body["session"]).unwrap();
    let message = format!("An email containing an link to delete your OLMMCC account has been sent to {}. Please check your inbox, including the spam folder, for the link. It may take a few minutes to receive the email.", session.get("email").unwrap());
    j_ok(json!({ "message": message }))
}

fn get_column_types(table: &str) -> Vec<String> {
    let mut column_types = Vec::new();
    for column in get_column_details(table) {
        column_types.push(from_value::<String>(column[1].clone()));
    }
    column_types
}

pub fn get_database(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            let mut column_names = Vec::new();
            for column in get_column_details(body["table"]) {
                column_names.push(from_value::<String>(column[0].clone()));
            }
            let mut processed_rows = Vec::new();
            let column_types = get_column_types(body["table"]);
            for row in get_all_rows(body["table"]) {
                let mut new_row = Vec::new();
                for i in 0..row.len() {
                    push_value(&column_types[i], row[i].clone(), &mut new_row);
                }
                processed_rows.push(new_row);
            }
            return j_ok(
                json!({"columns" : column_names, "rows" : processed_rows, "types" : column_types}),
            );
        }
    }
    ok("")
}

fn push_value(column_type: &str, value: mysql::Value, vec: &mut Vec<String>) {
    if column_type.contains("date") {
        vec.push(from_value::<NaiveDate>(value).to_string())
    } else if column_type.contains("int") {
        vec.push(from_value::<i32>(value).to_string())
    } else {
        vec.push(from_value::<String>(value).to_string());
    }
}

pub fn get_row_titles(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            let mut titles: Vec<String> = Vec::new();
            for title in get_some(body["table"], "title") {
                titles.push(from_value(title[0].clone()));
            }
            return j_ok(json!({"table" : body["table"], "titles" : titles}));
        }
    }
    ok("")
}

fn return_row(table: &str, id: i32) -> Vec<String> {
    let row = get_like(table, "id", &id.to_string())[0].clone();
    let mut formatted_row = Vec::new();
    let column_types = get_column_types(table);
    for i in 0..row.len() {
        push_value(&column_types[i], row[i].clone(), &mut formatted_row);
    }
    formatted_row
}

pub fn move_row_to_end(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            let new_id = get_max_id(body["table"]) + 1;
            change_row_where(body["table"], "id", body["id"], "id", &new_id.to_string());
            let message = format!("Successfully moved row {} to end.", body["id"]);
            return j_ok(
                json!({"success" : true, "message" : message, "row" : return_row(body["table"], new_id), "old_id" : body["id"]}),
            );
        }
    }
    ok("")
}

pub fn move_row_to_start(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            let new_id = get_min_id(body["table"]) - 1;
            change_row_where(body["table"], "id", body["id"], "id", &new_id.to_string());
            let message = format!("Successfully moved row {} to start.", body["id"]);
            return j_ok(
                json!({"success" : true, "message" : message, "row" : return_row(body["table"], new_id), "old_id" : body["id"]}),
            );
        }
    }
    ok("")
}

pub fn delete_row(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            delete_row_where(body["table"], "id", body["id"]);
            let message = format!("Successfully deleted row {}.", body["id"]);
            return j_ok(json!({"success" : true, "message" : message, "id" : body["id"]}));
        }
    }
    ok("")
}

pub fn add_row(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            let names = serde_json::from_str(body["names"]).unwrap();
            let values = serde_json::from_str(body["values"]).unwrap();
            if let Err(e) = insert_row(body["table"], names, values) {
                return j_ok(json!({"success" : false, "message" : e}));
            } else {
                let row_id = get_max_id(body["table"]);
                let message = format!("Successfully added row {}.", row_id);
                return j_ok(
                    json!({"success" : true, "message" : message, "row" : return_row(body["table"], row_id)}),
                );
            }
        }
    }
    ok("")
}

pub fn change_row(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            change_row_where(body["table"], "id", body["id"], body["name"], body["value"]);
            return ok(&format!("Successfully updated row {}.", body["id"]));
        }
    }
    ok("")
}

pub fn get_gmail_auth_url(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            let mut file = fs::File::open("/home/justus/client_secret.json").unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            let json: Value = serde_json::from_str(&contents).unwrap();
            return ok(&format!(
                "https://accounts.google.com/o/oauth2/v2/auth?scope=https://mail.google.com/&include_granted_scopes=true&prompt=consent&redirect_uri=https://www.olmmcc.tk/admin/email/&response_type=code&client_id={}&access_type=offline", 
                json["client_id"].as_str().unwrap(),
            ));
        }
    }
    ok("")
}

pub fn send_gmail_code(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            let mut file = fs::File::open("/home/justus/client_secret.json").unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            let json: Value = serde_json::from_str(&contents).unwrap();
            let mut hash = HashMap::new();
            hash.insert("code", body["code"]);
            hash.insert("access_type", "offline");
            hash.insert("client_id", json["client_id"].as_str().unwrap());
            hash.insert("client_secret", json["client_secret"].as_str().unwrap());
            hash.insert("redirect_uri", "https://www.olmmcc.tk/admin/email/");
            hash.insert("grant_type", "authorization_code");
            let request =
                http::make_secure_request("https://www.googleapis.com/oauth2/v4/token", hash);
            let request_json: Value = serde_json::from_str(&request).unwrap();
            let refresh_token = request_json["refresh_token"].as_str().unwrap();
            println!("{:?}", session.get("email"));
            let email = &session.get("email").unwrap();
            if row_exists("admin", "email", email) {
                change_row_where("admin", "email", email, "refresh_token", refresh_token);
            } else {
                insert_row(
                    "admin",
                    vec!["email", "refresh_token"],
                    vec![email, refresh_token],
                )
                .unwrap();
            }
            println!("{:?}", request_json);
        }
    }
    ok("")
}
