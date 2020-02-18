use chrono::NaiveDate;
use mysql::from_value;
use serde::Serialize;
use serde_json::{json, Map, Value};
use session::Session;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::iter;
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
pub fn get_songs() -> String {
    let mut expiry = 0;
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut article: Vec<SongArticle> = get_all_rows("articles", true)
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
    if let Some(t) = check_email(&email) {
        return message(t);
    }
    insert_row(
        "users",
        vec!["email", "subscription_policy"],
        vec![&email, "1"],
    )
    .unwrap();
    let mut session = Session::new(30, 100);
    match refresh_user_session(&mut session, "email", email.clone(), "0") {
        Some(message) => j_ok(json!({ "message": message })),
        None => j_ok(json!({"session" : session.get_id(), "email": email})),
    }
}

pub fn login(body: HashMap<&str, &str>) -> String {
    let email = body["email"].to_lowercase();
    let mut session = Session::new(30, 100);
    match refresh_user_session(&mut session, "email", email.clone(), "0") {
        Some(message) => j_ok(json!({ "message": message })),
        None => j_ok(json!({"session" : session.get_id(), "email": email})),
    }
}

pub fn admin_login(body: HashMap<&str, &str>) -> String {
    let email = body["email"].to_lowercase();
    let mut session = Session::new(30, 100);
    match refresh_admin_session(&mut session, "email", email, Some(body["password"])) {
        Some(message) => j_ok(json!({ "message": message })),
        None => j_ok(json!({"session" : session.get_id()})),
    }
}

fn refresh_user_session(
    session: &mut Session,
    key: &str,
    value: String,
    verified: &str,
) -> Option<String> {
    session.clear();
    let users = get_like("users", key, &value);
    if let Some(user) = users.iter().next() {
        session.set("id", from_value::<i32>(user[1].clone()).to_string());
        if verified == "1" {
            session
                .set("verified", "1".to_string())
                .set("email", from_value(user[0].clone()))
                .set("admin", 0.to_string())
                .set(
                    "subscription_policy",
                    from_value::<i32>(user[2].clone()).to_string(),
                );
            None
        } else {
            session
                .set("verified", "0".to_string())
                .set("not_verified_email", from_value(user[0].clone()));
            None
        }
    } else {
        let admin = get_like("admin", key, &value);
        if let Some(admin) = admin.iter().next() {
            session
                .set("id", from_value::<i32>(admin[2].clone()).to_string())
                .set("not_verified_admin", 1.to_string())
                .set("verified", "0".to_string())
                .set("not_verified_email", from_value(admin[0].clone()));
            None
        } else {
            Some("This email address is not registered. Please create a new account.".to_string())
        }
    }
}

fn refresh_admin_session(
    session: &mut Session,
    key: &str,
    value: String,
    password: Option<&str>,
) -> Option<String> {
    session.clear();
    let users = get_like("admin", key, &value);
    if let Some(user) = users.iter().next() {
        if let Some(p) = password {
            if !hash_match(p, &from_value::<String>(user[1].clone())) {
                return Some("Wrong password, please try again.".to_string());
            }
        }
        session.set("id", from_value::<i32>(user[2].clone()).to_string());
        session
            .set("email", from_value(user[0].clone()))
            .set("admin", 1.to_string())
            .set(
                "subscription_policy",
                from_value::<i32>(user[3].clone()).to_string(),
            );
        None
    } else {
        Some("This account is not an administrator account.".to_string())
    }
}

pub fn get_account(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("verified").unwrap_or_default() == "1"
            || session.get("admin").unwrap_or_default() == "1"
        {
            const ALLOWED_VARS: &[&str] = &["email", "admin", "subscription_policy"];
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
    j_ok(json!({}))
}

pub fn refresh(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        let id = session.get("id").unwrap();
        let verified = &session.get("verified").unwrap();
        refresh_user_session(&mut session, "id", id, verified).unwrap();
    }
    j_ok(json!({}))
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
    session.set("subscription_policy", body["subscription"].to_string());
    j_ok(json!({
        "message" : SUBSCRIPTION_MESSAGES[body["subscription"].parse::<usize>().unwrap()]
    }))
}

fn queue_change_email(session: &mut Session, new_email: &str) -> String {
    let email = session.get("email").unwrap();
    let email_change_code = generate_verification_code();
    session.set("email_change_code", email_change_code.clone());
    session.set("new_email", new_email.to_string());
    gmail::send_email(
            vec!(email.clone()),
            "Verify your Email Change Request",
            &format!("Hello,\r\nYou requested a change of your email address to {}. Please copy this code and return to OLMMCC's website: {}\r\n\r\nThis message was sent by the OLMMCC automated system. If you did not make this request please contact justus@olmmcc.tk", new_email, email_change_code),
            get_access_token().as_str(),
        );
    email
}

pub fn send_change_email(body: HashMap<&str, &str>) -> String {
    // An email needs to be added to the queue here
    let mut session = Session::from_id(body["session"]).unwrap();
    if session.get("verified").unwrap() == "1" {
        if let Some(t) = check_email(body["email"]) {
            return message(t);
        }
        j_ok(json!({ "success": true, "email": queue_change_email(&mut session, body["email"]) }))
    } else {
        j_ok(json!({ "success": false }))
    }
}

pub fn change_email(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        let admin = session.get("admin").unwrap() == "1";
        if admin || session.get("verified").unwrap() == "1" {
            if session.get("email_change_code").unwrap() == body["code"] {
                let id = session.get("id").unwrap();
                let new_email = session.get("new_email").unwrap();
                if admin {
                    change_row_where("admin", "id", &id, "email", &new_email);
                    refresh_admin_session(&mut session, "id", id, None);
                } else {
                    change_row_where("users", "id", &id, "email", &new_email);
                    refresh_user_session(&mut session, "id", id, "0");
                }
                return j_ok(json!({ "success": true }));
            }
        }
        println!("{:?} {}", session.get("email_change_code"), body["code"]);
    }
    j_ok(json!({"success": false}))
}

pub fn send_delete_email(body: HashMap<&str, &str>) -> String {
    // An email needs to be added to the queue here
    let mut session = Session::from_id(body["session"]).unwrap();
    if session.get("admin").unwrap() == "1" || session.get("verified").unwrap() == "1" {
        j_ok(json!({ "success": true, "email": queue_delete_email(&mut session) }))
    } else {
        j_ok(json!({ "success": false }))
    }
}

fn queue_delete_email(session: &mut Session) -> String {
    let email = session.get("email").unwrap();
    let delete_code = generate_verification_code();
    session.set("delete_code", delete_code.clone());
    gmail::send_email(
        vec!(email.clone()),
        "Verify your Account Deletion Request",
        &format!("Hello,\r\nYou requested a deletion of your OLMMCC account. Please copy this code and return to OLMMCC's website: {}\r\n\r\nThis message was sent by the OLMMCC automated system. If you did not make this request please contact justus@olmmcc.tk", delete_code),
        get_access_token().as_str(),
    );
    email
}

pub fn delete_account(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        let admin = session.get("admin").unwrap() == "1";
        if admin || session.get("verified").unwrap() == "1" {
            if session.get("delete_code").unwrap() == body["code"] {
                let id = session.get("id").unwrap();
                if admin {
                    delete_row_where("admin", "id", &id);
                } else {
                    delete_row_where("users", "id", &id);
                }
                return j_ok(json!({ "success": true }));
            }
        }
    }
    j_ok(json!({"success": false}))
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
            for row in get_all_rows(body["table"], true) {
                let mut new_row = Vec::new();
                for i in 0..row.len() {
                    push_value(&column_types[i], row[i].clone(), &mut new_row);
                }
                processed_rows.push(new_row);
            }
            return j_ok(
                json!({"success": true, "columns" : column_names, "rows" : processed_rows, "types" : column_types}),
            );
        }
    }
    j_ok(json!({"success": false}))
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
            if body["table"] == "admin" {
                if session.get("id").unwrap() == body["id"] {
                    return j_ok(
                        json!({"success" : false, "authorized" : true, "email": queue_delete_email(&mut session)}),
                    );
                } else {
                    return j_ok(json!({"success" : false, "authorized": false}));
                }
            } else {
                delete_row_where(body["table"], "id", body["id"]);
                let message = format!("Successfully deleted row {}.", body["id"]);
                return j_ok(json!({"success" : true, "message" : message, "id" : body["id"]}));
            }
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
            if body["table"] == "admin" {
                if session.get("id").unwrap() == body["id"] {
                    if body["name"] == "email" {
                        return j_ok(
                            json!({"success" : false, "authorized" : true, "email": queue_change_email(&mut session, body["value"])}),
                        );
                    }
                } else {
                    return j_ok(json!({"success" : false, "authorized": false}));
                }
            }
            change_row_where(body["table"], "id", body["id"], body["name"], body["value"]);
            return j_ok(json!({
                "success": true,
                "message": &format!("Successfully updated row {}.", body["id"])
            }));
        }
    }
    ok("")
}

pub fn get_gmail_auth_url(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            let mut file = File::open("/home/justus/client_secret.json").unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            let json: Value = serde_json::from_str(&contents).unwrap();
            return j_ok(json!({
                "url": &format!(
                "https://accounts.google.com/o/oauth2/v2/auth?scope=https://mail.google.com/&include_granted_scopes=true&prompt=consent&redirect_uri=https://www.olmmcc.tk/admin/email/&response_type=code&client_id={}&access_type=offline", 
                json["client_id"].as_str().unwrap(),
            )
            }));
        }
    }
    j_ok(json!({"url": ""}))
}

pub fn send_gmail_code(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            let refresh_token = gmail::get_refresh_token(body["code"]);
            let email = &session.get("email").unwrap();
            if row_exists("admin", "email", email) {
                change_row_where("admin", "email", email, "refresh_token", &refresh_token);
            } else {
                insert_row(
                    "admin",
                    vec!["email", "refresh_token"],
                    vec![email, &refresh_token],
                )
                .unwrap();
            }
        }
    }
    j_ok(json!({}))
}

pub fn is_gmail_working(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            if get_refresh_token() == mysql::Value::NULL {
                return j_ok(json!({"working": false}));
            } else {
                return j_ok(json!({"working": true}));
            }
        }
    }
    j_ok(json!({}))
}

fn get_refresh_token() -> mysql::Value {
    let mut return_token = mysql::Value::NULL;
    for row in get_all_rows("admin", false) {
        let token = row[4].clone();
        if token != mysql::Value::Bytes(vec![]) {
            return_token = token;
            break;
        }
    }
    return_token
}

fn get_access_token() -> String {
    let refresh_token = get_refresh_token();
    gmail::get_access_token(&from_value::<String>(refresh_token))
}

fn generate_verification_code() -> String {
    let mut rng = thread_rng();
    iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(16)
        .collect()
}

pub fn send_login_email(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("verified").unwrap() == "0" {
            let email = session.get("not_verified_email").unwrap();
            let verification_code = generate_verification_code();
            session.set("verification_code", verification_code.clone());
            gmail::send_email(
                vec!(email),
                "Verify Your Identity",
                &format!("Hello,\r\nTo verify your identity, please copy this code and return to OLMMCC's website: {}\r\n\r\nThis message was sent by the OLMMCC automated system. If you received it in error please contact justus@olmmcc.tk", verification_code),
                get_access_token().as_str(),
            );
            return j_ok(json!({ "success": true }));
        }
    }
    j_ok(json!({"success": false}))
}

pub fn hash_password(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            return j_ok(json!({"hash": hash(body["password"])}));
        }
    }
    ok("")
}

pub fn verify_account(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("verified").unwrap() == "0" {
            if session.get("verification_code").unwrap() == body["code"] {
                let email = session.get("not_verified_email").unwrap();
                if session.get("not_verified_admin").unwrap_or_default() == "1" {
                    refresh_admin_session(&mut session, "email", email, None);
                } else {
                    refresh_user_session(&mut session, "email", email, "1");
                }
                return j_ok(json!({ "success": true }));
            }
        }
    }
    j_ok(json!({"success": false}))
}

pub fn send_email(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]) {
        if session.get("admin").unwrap() == "1" {
            let mut emails = vec!();
            if body["recipients"] == "all_users" {
                for row in get_some("users", "email") {
                    emails.push(from_value::<String>(row[0].clone()));
                }
                for row in get_some("admin", "email") {
                    emails.push(from_value::<String>(row[0].clone()));
                }
            } else {
                emails.push(body["recipient"].to_string());
            }
            gmail::send_email(
                emails,
                body["subject"],
                body["body"],
                get_access_token().as_str(),
            );
            return j_ok(json!({ "success": true }));
        }
    }
    j_ok(json!({ "success": false }))
}
