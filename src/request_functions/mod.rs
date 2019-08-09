use chrono::NaiveDate;
use htmlescape::decode_html;
use mysql::from_value;
use serde::Serialize;
use serde_json::json;
use session::Session;

use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

mod account_validation;
use account_validation::*;

mod database_functions;
use database_functions::*;

use crate::{hash, hash_match, message, ok};

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
        get_like("pages", "topnav_id", body.get("page").unwrap())[0][1].clone(),
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
        None => ok(&json!({"title" : ""}).to_string()),
    }
}

pub fn get_image_list() -> String {
    let paths: Vec<String> = fs::read_dir("/srv/http/images/")
        .unwrap()
        .map(|x| x.unwrap().file_name().into_string().unwrap())
        .collect();
    let json = json!({ "images": paths });
    ok(&json.to_string())
}

pub fn get_calendar_events(body: HashMap<&str, &str>) -> String {
    let result: Vec<CalendarEvent> = get_like("calendar", "date", body.get("year_month").unwrap())
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
    let email = body.get("email").unwrap().to_lowercase();
    let username = body.get("username").unwrap();
    let password_one = body.get("password1").unwrap();
    let password_two = body.get("password2").unwrap();
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
    );
    let json = json!({
        "url" : "/login"
    });
    ok(&json.to_string())
}

pub fn login(body: HashMap<&str, &str>) -> String {
    let email = body.get("email").unwrap().to_lowercase();
    let iter = get_like("users", "email", &email);
    if let Some(user) = iter.iter().next() {
        if hash_match(
            body.get("password").unwrap(),
            &from_value::<String>(user[2].clone()),
        ) {
            if from_value::<i32>(user[4].clone()) == 1 {
                let mut session = Session::new(30, 100);
                session
                    .set("id", from_value::<i32>(user[3].clone()).to_string())
                    .set("verified", "1".to_string())
                    .set(
                        "invalid_email",
                        from_value::<i32>(user[7].clone()).to_string(),
                    )
                    .set("email", email)
                    .set("username", from_value(user[1].clone()))
                    .set("admin", from_value::<i32>(user[5].clone()).to_string())
                    .set(
                        "subscription_policy",
                        from_value::<i32>(user[6].clone()).to_string(),
                    )
                    .set("notification", "Successfully logged in!".to_string());
                ok(&json!({"url" : "/", "session" : session.get_id()}).to_string())
            } else {
                ok(
                    &json!({"url" : "", "message" : "This account has not been verified."})
                        .to_string(),
                )
            }
        } else {
            ok(&json!({"url" : "", "message" : "Wrong password, please try again."}).to_string())
        }
    } else {
        ok(&json!({"url" : "", "message" : "Wrong email, please try again."}).to_string())
    }
}
