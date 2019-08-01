use mysql::params;
use chrono::NaiveDate;
use serde::Serialize;
use serde_json::json;
use htmlescape::decode_html;

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;

mod account_validation;
use account_validation::*;

mod database_functions;
use database_functions::*;

use crate::{get_mysql_conn, ok, message, hash};

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
    expiry: i64,
    songs: Vec<Song>
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
    ok(&decode_html(
        &mysql::from_value::<String>
            (get_row("pages", "topnav_id", body.get("page").unwrap())[0][1].clone())
    ).unwrap())
}
pub fn get_songs() -> String {
    let mut conn = get_mysql_conn();
    let mut expiry = 0;
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let mut result: Vec<SongArticle> = conn
        .prep_exec("SELECT * FROM articles", ()).unwrap()
        .map(|row| {
            let (_, title, text, expiry) = mysql::from_row::<(i32, _, _, NaiveDate)>(row.unwrap());
            SongArticle {
                title,
                text,
                expiry: expiry.and_hms(0, 0, 0).timestamp(),
                songs: Vec::new(),
            }
        })
        .filter(|article| {
            if article.expiry > expiry && current_time < article.expiry as u64 {
                expiry = article.expiry;
                true
            } else {
                false
            }
        })
        .collect();
    match result.pop() {
        Some(mut t) => {
            let result: Vec<Song> = conn
                .prep_exec(
                    "SELECT * FROM songs WHERE article LIKE :a", 
                    params!("a" => &t.title)
                ).unwrap()
                .map(|row| {
                    let (_, name, link, role, _) = 
                        mysql::from_row::<(i32, _, _, _, String)>(row.unwrap());
                    Song {
                        name,
                        link,
                        role,
                    }
                })
                .collect();
            t.songs = result;
            ok(&serde_json::to_string(&t).unwrap())
        },
        None => {
            ok("{\"title\": \"\"}")
        }
    }
}

pub fn get_image_list() -> String {
    let paths: Vec<String> = fs::read_dir("/srv/http/images/").unwrap()
        .map(|x| {x.unwrap().file_name().into_string().unwrap()})
        .collect();
    let json = json!({
        "images" : paths
    });
    ok(&json.to_string())
}

pub fn get_calendar_events(body: HashMap<&str, &str>) -> String {
    let mut conn = get_mysql_conn();
    let result: Vec<CalendarEvent> = conn
        .prep_exec(
            "SELECT * FROM calendar WHERE date LIKE :a", 
            params!("a" => body.get("year_month").unwrap())
        ).unwrap()
        .map(|row| {
            let (id, title, date, start_time, end_time, notes) = 
                mysql::from_row::<(_, _, NaiveDate, _, _, _)>(row.unwrap());
            CalendarEvent {
                id,
                title,
                date: date.format("%Y-%m-%d").to_string(),
                start_time,
                end_time,
                notes,
            }
        })
        .collect();
    ok(&serde_json::to_string(&result).unwrap())
}

pub fn signup(body: HashMap<&str, &str>) -> String {
    let email = body.get("email").unwrap().to_lowercase();
    let username = body.get("username").unwrap();
    let password_one = body.get("password1").unwrap();
    let password_two = body.get("password2").unwrap();
    if let Some(t) = check_passwords(password_one, password_two) { return message(t); }
    if let Some(t) = check_email(&email) { return message(t); }
    if let Some(t) = check_username(username) { return message(t); }
    let mut conn = get_mysql_conn();
    conn.prep_exec(
        "INSERT INTO users (email, username, password, verified, admin, subscription_policy, invalid_email) VALUES (:email, :username, :password, 0, 0, 1, 0)", 
        params!(
            "email" => email,
            "username" => username,
            "password" => hash(password_one),
        )
    ).unwrap();
    let json = json!({
        "url" : "/login"
    });
    ok(&json.to_string())
}