use scrypt::{scrypt_check, scrypt_simple, ScryptParams};
use chrono::NaiveDate;
use mysql_async::from_value;
use serde::Serialize;
use serde_json::{json, Map, Value};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use session::Session;

use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::iter;
use std::time::{SystemTime, UNIX_EPOCH};

use account_validation::*;
use database_functions::*;
mod account_validation;
mod database_functions;

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

pub async fn formulate_response(url: &str, body: HashMap<&str, &str>) -> String {
    match url {
        "/get_songs" => get_songs().await,
        "/hash_password" => hash_password(body).await,
        "/get_image_list" => get_image_list(),
        "/get_calendar_events" => get_calendar_events(body).await,
        "/signup" => signup(body).await,
        "/login" => login(body).await,
        "/admin_login" => admin_login(body).await,
        "/kill_session" => kill_session(body).await,
        "/get_account" => get_account(body).await,
        "/refresh" => refresh(body).await,
        "/change_subscription" => change_subscription(body).await,
        "/send_change_email" => send_change_email(body).await,
        "/send_delete_email" => send_delete_email(body).await,
        "/change_email" => change_email(body).await,
        "/delete_account" => delete_account(body).await,
        "/get_database" => get_database(body).await,
        "/get_row_titles" => get_row_titles(body).await,
        "/move_row_to_end" => move_row_to_end(body).await,
        "/move_row_to_start" => move_row_to_start(body).await,
        "/delete_row" => delete_row(body).await,
        "/add_row" => add_row(body).await,
        "/change_row" => change_row(body).await,
        "/get_gmail_auth_url" => get_gmail_auth_url(body).await,
        "/is_gmail_working" => is_gmail_working(body).await,
        "/send_gmail_code" => send_gmail_code(body).await,
        "/verify_account" => verify_account(body).await,
        "/send_email" => send_email(body).await,
        _ => message(&format!("The provided url {} could not be resolved.", url))
    }
}

fn message(message: &str) -> String {
    json!({ "message": message }).to_string()
}
fn hash(to_hash: &str) -> String {
    scrypt_simple(to_hash, &ScryptParams::new(12, 8, 1).unwrap()).unwrap()
}
fn hash_match(password: &str, hash: &str) -> bool {
    scrypt_check(password, hash).is_ok()
}

pub async fn get_songs() -> String {
    let mut expiry = 0;
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut article: Vec<SongArticle> = get_all_rows("articles", true).await
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
            t.songs = get_like("songs", "article", &t.title).await
                .into_iter()
                .map(|x| Song {
                    name: from_value(x[1].clone()),
                    link: from_value(x[2].clone()),
                    role: from_value(x[3].clone()),
                })
                .collect();
            serde_json::to_string(&t).unwrap()
        }
        None => json!({"title" : ""}).to_string(),
    }
}

pub fn get_image_list() -> String {
    let paths: Vec<String> = fs::read_dir("/srv/http/images/")
        .unwrap()
        .map(|x| x.unwrap().file_name().into_string().unwrap())
        .collect();
    json!({ "images": paths }).to_string()
}

pub async fn get_calendar_events(body: HashMap<&str, &str>) -> String {
    let result: Vec<CalendarEvent> = get_like("calendar", "date", body["year_month"]).await
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
    serde_json::to_string(&result).unwrap()
}

pub async fn signup(body: HashMap<&str, &str>) -> String {
    let email = body["email"].to_lowercase();
    if let Some(t) = check_email(&email).await {
        return message(t);
    }
    insert_row(
        "users",
        vec!["email", "subscription_policy"],
        vec![&email, "1"],
    ).await
    .unwrap();
    let mut session = Session::new(30, 100).await;
    match refresh_user_session(&mut session, "email", email.clone(), "0").await {
        Some(t) => message(&t),
        None => send_login_email(&mut session).await,
    }
}

pub async fn login(body: HashMap<&str, &str>) -> String {
    let email = body["email"].to_lowercase();
    let mut session = Session::new(30, 100).await;
    match refresh_user_session(&mut session, "email", email.clone(), "0").await {
        Some(t) => message(&t),
        None => send_login_email(&mut session).await,
    }
}

async fn send_login_email(session: &mut Session) -> String {
    let email = session.get("not_verified_email").await.unwrap();
    let verification_code = generate_verification_code();
    session.set("verification_code", verification_code.clone()).await;
    let access_token = get_access_token().await;
    let body = format!("Hello,\r\nTo verify your identity, please copy this code and return to OLMMCC's website: {}\r\n\r\nThis message was sent by the OLMMCC automated system. If you received it in error please contact justus@olmmcc.tk", verification_code);
    gmail::send_email(
        vec!(email.clone()),
        "Verify Your Identity",
        &body,
        &access_token,
    ).await;
    json!({"session" : session.get_id(), "email": email}).to_string()
}

pub async fn admin_login(body: HashMap<&str, &str>) -> String {
    let email = body["email"].to_lowercase();
    let mut session = Session::new(30, 100).await;
    match refresh_admin_session(&mut session, "email", email, Some(body["password"])).await {
        Some(t) => message(&t),
        None => json!({"session" : session.get_id()}).to_string(),
    }
}

async fn refresh_user_session(
    session: &mut Session,
    key: &str,
    value: String,
    verified: &str,
) -> Option<String> {
    session.clear().await;
    let users = get_like("users", key, &value).await;
    if let Some(user) = users.iter().next() {
        session.set("id", from_value::<i32>(user[1].clone()).to_string()).await;
        if verified == "1" {
            session
                .set("verified", "1".to_string()).await
                .set("email", from_value(user[0].clone())).await
                .set("admin", 0.to_string()).await
                .set(
                    "subscription_policy",
                    from_value::<i32>(user[2].clone()).to_string(),
                ).await;
            None
        } else {
            session
                .set("verified", "0".to_string()).await
                .set("not_verified_email", from_value(user[0].clone())).await;
            None
        }
    } else {
        let admin = get_like("admin", key, &value).await;
        if let Some(admin) = admin.iter().next() {
            session
                .set("id", from_value::<i32>(admin[2].clone()).to_string()).await
                .set("not_verified_admin", 1.to_string()).await
                .set("verified", "0".to_string()).await
                .set("not_verified_email", from_value(admin[0].clone())).await;
            None
        } else {
            Some("This email address is not registered. Please create a new account.".to_string())
        }
    }
}

async fn refresh_admin_session(
    session: &mut Session,
    key: &str,
    value: String,
    password: Option<&str>,
) -> Option<String> {
    session.clear().await;
    let users = get_like("admin", key, &value).await;
    if let Some(user) = users.iter().next() {
        if let Some(p) = password {
            if !hash_match(p, &from_value::<String>(user[1].clone())) {
                return Some("Wrong password, please try again.".to_string());
            }
        }
        session.set("id", from_value::<i32>(user[2].clone()).to_string()).await;
        session
            .set("email", from_value(user[0].clone())).await
            .set("admin", 1.to_string()).await
            .set(
                "subscription_policy",
                from_value::<i32>(user[3].clone()).to_string(),
            ).await;
        None
    } else {
        Some("This account is not an administrator account.".to_string())
    }
}

pub async fn get_account(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("verified").await.unwrap_or_default() == "1"
            || session.get("admin").await.unwrap_or_default() == "1"
        {
            const ALLOWED_VARS: &[&str] = &["email", "admin", "subscription_policy"];
            let mut map = Map::new();
            for var in ALLOWED_VARS {
                if body["details"].contains(var) {
                    map.insert(var.to_string(), Value::String(session.get(var).await.unwrap()));
                }
            }
            return serde_json::to_string(&map).unwrap();
        }
    }
    json!({"session" : "none"}).to_string()
}

pub async fn kill_session(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        session.delete().await;
    }
    json!({}).to_string()
}

pub async fn refresh(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        let id = session.get("id").await.unwrap();
        let verified = &session.get("verified").await.unwrap();
        refresh_user_session(&mut session, "id", id, verified).await.unwrap();
    }
    json!({}).to_string()
}

pub async fn change_subscription(body: HashMap<&str, &str>) -> String {
    const SUBSCRIPTION_MESSAGES: &[&str] = &[
        "You are now unsubscribed from receiving emails.",
        "You are now subscribed to receive emails.",
        "You are now subscribed to receive emails and reminders.",
    ];
    let mut session = Session::from_id(body["session"]).await.unwrap();
    if let Some(t) = check_subscription(body["subscription"]) {
        return message(&t);
    }
    change_row_where(
        "users",
        "id",
        &session.get("id").await.unwrap(),
        "subscription_policy",
        body["subscription"],
    ).await;
    session.set("subscription_policy", body["subscription"].to_string()).await;
    message(SUBSCRIPTION_MESSAGES[body["subscription"].parse::<usize>().unwrap()])
}

async fn queue_change_email(session: &mut Session, new_email: &str) -> String {
    let email = session.get("email").await.unwrap();
    let email_change_code = generate_verification_code();
    session.set("email_change_code", email_change_code.clone()).await;
    session.set("new_email", new_email.to_string()).await;
    let body = format!("Hello,\r\nYou requested a change of your email address to {}. Please copy this code and return to OLMMCC's website: {}\r\n\r\nThis message was sent by the OLMMCC automated system. If you did not make this request please contact justus@olmmcc.tk", new_email, email_change_code);
    let access_token = get_access_token().await;
    gmail::send_email(
            vec!(email.clone()),
            "Verify your Email Change Request",
            &body,
            &access_token,
        ).await;
    email
}

pub async fn send_change_email(body: HashMap<&str, &str>) -> String {
    // An email needs to be added to the queue here
    let mut session = Session::from_id(body["session"]).await.unwrap();
    if session.get("verified").await.unwrap() == "1" {
        if let Some(t) = check_email(body["email"]).await {
            return message(t);
        }
        json!({ "success": true, "email": queue_change_email(&mut session, body["email"]).await }).to_string()
    } else {
        json!({ "success": false }).to_string()
    }
}

pub async fn change_email(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        let admin = session.get("admin").await.unwrap() == "1";
        if admin || session.get("verified").await.unwrap() == "1" {
            if session.get("email_change_code").await.unwrap() == body["code"] {
                let id = session.get("id").await.unwrap();
                let new_email = session.get("new_email").await.unwrap();
                if admin {
                    change_row_where("admin", "id", &id, "email", &new_email).await;
                    refresh_admin_session(&mut session, "id", id, None).await;
                } else {
                    change_row_where("users", "id", &id, "email", &new_email).await;
                    refresh_user_session(&mut session, "id", id, "0").await;
                }
                return json!({ "success": true }).to_string();
            }
        }
        println!("{:?} {}", session.get("email_change_code").await, body["code"]);
    }
    json!({"success": false}).to_string()
}

pub async fn send_delete_email(body: HashMap<&str, &str>) -> String {
    // An email needs to be added to the queue here
    let mut session = Session::from_id(body["session"]).await.unwrap();
    if session.get("admin").await.unwrap() == "1" || session.get("verified").await.unwrap() == "1" {
        json!({ "success": true, "email": queue_delete_email(&mut session).await }).to_string()
    } else {
        json!({ "success": false }).to_string()
    }
}

async fn queue_delete_email(session: &mut Session) -> String {
    let email = session.get("email").await.unwrap();
    let delete_code = generate_verification_code();
    session.set("delete_code", delete_code.clone()).await;
    let body = format!("Hello,\r\nYou requested a deletion of your OLMMCC account. Please copy this code and return to OLMMCC's website: {}\r\n\r\nThis message was sent by the OLMMCC automated system. If you did not make this request please contact justus@olmmcc.tk", delete_code);
    let access_token = get_access_token().await;
    gmail::send_email(
        vec!(email.clone()),
        "Verify your Account Deletion Request",
        &body,
        &access_token,
    ).await;
    email
}

pub async fn delete_account(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        let admin = session.get("admin").await.unwrap() == "1";
        if admin || session.get("verified").await.unwrap() == "1" {
            if session.get("delete_code").await.unwrap() == body["code"] {
                let id = session.get("id").await.unwrap();
                if admin {
                    delete_row_where("admin", "id", &id).await;
                } else {
                    delete_row_where("users", "id", &id).await;
                }
                return json!({ "success": true }).to_string();
            }
        }
    }
    json!({"success": false}).to_string()
}

async fn get_column_types(table: &str) -> Vec<String> {
    let mut column_types = Vec::new();
    for column in get_column_details(table).await {
        column_types.push(from_value::<String>(column[1].clone()));
    }
    column_types
}

pub async fn get_database(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("admin").await.unwrap() == "1" {
            let mut column_names = Vec::new();
            for column in get_column_details(body["table"]).await {
                column_names.push(from_value::<String>(column[0].clone()));
            }
            let mut processed_rows = Vec::new();
            let column_types = get_column_types(body["table"]).await;
            for row in get_all_rows(body["table"], true).await {
                let mut new_row = Vec::new();
                for i in 0..row.len() {
                    push_value(&column_types[i], row[i].clone(), &mut new_row);
                }
                processed_rows.push(new_row);
            }
            return 
                json!({"success": true, "columns" : column_names, "rows" : processed_rows, "types" : column_types}).to_string();
        }
    }
    json!({"success": false}).to_string()
}

fn push_value(column_type: &str, value: mysql_async::Value, vec: &mut Vec<String>) {
    if column_type.contains("date") {
        vec.push(from_value::<NaiveDate>(value).to_string())
    } else if column_type.contains("int") {
        vec.push(from_value::<i32>(value).to_string())
    } else {
        vec.push(from_value::<String>(value).to_string());
    }
}

pub async fn get_row_titles(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("admin").await.unwrap() == "1" {
            let mut titles: Vec<String> = Vec::new();
            for title in get_some(body["table"], "title").await {
                titles.push(from_value(title[0].clone()));
            }
            return json!({"table" : body["table"], "titles" : titles}).to_string();
        }
    }
    json!({}).to_string()
}

async fn return_row(table: &str, id: i32) -> Vec<String> {
    let row = get_like(table, "id", &id.to_string()).await[0].clone();
    let mut formatted_row = Vec::new();
    let column_types = get_column_types(table).await;
    for i in 0..row.len() {
        push_value(&column_types[i], row[i].clone(), &mut formatted_row);
    }
    formatted_row
}

pub async fn move_row_to_end(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("admin").await.unwrap() == "1" {
            let new_id = get_max_id(body["table"]).await + 1;
            change_row_where(body["table"], "id", body["id"], "id", &new_id.to_string()).await;
            let message = format!("Successfully moved row {} to end.", body["id"]);
            return 
                json!({"success" : true, "message" : message, "row" : return_row(body["table"], new_id).await, "old_id" : body["id"]}).to_string()
            ;
        }
    }
    json!({}).to_string()
}

pub async fn move_row_to_start(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("admin").await.unwrap() == "1" {
            let new_id = get_min_id(body["table"]).await - 1;
            change_row_where(body["table"], "id", body["id"], "id", &new_id.to_string()).await;
            let message = format!("Successfully moved row {} to start.", body["id"]);
            let row = return_row(body["table"], new_id).await;
            return 
                json!({"success" : true, "message" : message, "row" : row, "old_id" : body["id"]}).to_string();
        }
    }
    json!({}).to_string()
}

pub async fn delete_row(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("admin").await.unwrap() == "1" {
            if body["table"] == "admin" {
                if session.get("id").await.unwrap() == body["id"] {
                    return 
                        json!({"success" : false, "authorized" : true, "email": queue_delete_email(&mut session).await}).to_string();
                } else {
                    return json!({"success" : false, "authorized": false}).to_string();
                }
            } else {
                delete_row_where(body["table"], "id", body["id"]).await;
                let message = format!("Successfully deleted row {}.", body["id"]);
                return json!({"success" : true, "message" : message, "id" : body["id"]}).to_string();
            }
        }
    }
    json!({}).to_string()
}

pub async fn add_row(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("admin").await.unwrap() == "1" {
            let names = serde_json::from_str(body["names"]).unwrap();
            let values = serde_json::from_str(body["values"]).unwrap();
            if let Err(e) = insert_row(body["table"], names, values).await {
                return json!({"success" : false, "message" : e}).to_string();
            } else {
                let row_id = get_max_id(body["table"]).await;
                let message = format!("Successfully added row {}.", row_id);
                let row = return_row(body["table"], row_id).await;
                return 
                    json!({"success" : true, "message" : message, "row" : row}).to_string();
            }
        }
    }
    json!({}).to_string()
}

pub async fn change_row(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("admin").await.unwrap() == "1" {
            if body["table"] == "admin" {
                if session.get("id").await.unwrap() == body["id"] {
                    if body["name"] == "email" {
                        return 
                            json!({"success" : false, "authorized" : true, "email": queue_change_email(&mut session, body["value"]).await}).to_string();
                    }
                } else {
                    return json!({"success" : false, "authorized": false}).to_string();
                }
            }
            change_row_where(body["table"], "id", body["id"], body["name"], body["value"]).await;
            return json!({
                "success": true,
                "message": &format!("Successfully updated row {}.", body["id"])
            }).to_string();
        }
    }
    json!({}).to_string()
}

pub async fn get_gmail_auth_url(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("admin").await.unwrap() == "1" {
            let mut file = File::open("/home/justus/client_secret.json").unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            let json: Value = serde_json::from_str(&contents).unwrap();
            return json!({
                "url": &format!(
                "https://accounts.google.com/o/oauth2/v2/auth?scope=https://mail.google.com/&include_granted_scopes=true&prompt=consent&redirect_uri=https://www.olmmcc.tk/admin/email/&response_type=code&client_id={}&access_type=offline", 
                json["client_id"].as_str().unwrap(),
            )
            }).to_string();
        }
    }
    json!({"url": ""}).to_string()
}

pub async fn send_gmail_code(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("admin").await.unwrap() == "1" {
            let refresh_token = gmail::get_refresh_token(body["code"]);
            let email = &session.get("email").await.unwrap();
            if row_exists("admin", "email", email).await {
                change_row_where("admin", "email", email, "refresh_token", &refresh_token.await).await;
            } else {
                insert_row(
                    "admin",
                    vec!["email", "refresh_token"],
                    vec![email, &refresh_token.await],
                ).await
                .unwrap();
            }
        }
    }
    json!({}).to_string()
}

pub async fn is_gmail_working(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("admin").await.unwrap() == "1" {
            if get_refresh_token().await != mysql_async::Value::NULL {
                return json!({"working": true}).to_string();
            }
        }
    }
    json!({"working": false}).to_string()
}

async fn get_refresh_token() -> mysql_async::Value {
    let mut return_token = mysql_async::Value::NULL;
    for row in get_all_rows("admin", false).await {
        let token = row[4].clone();
        if token != mysql_async::Value::Bytes(vec![]) {
            return_token = token;
            break;
        }
    }
    return_token
}

async fn get_access_token() -> String {
    let refresh_token = get_refresh_token();
    gmail::get_access_token(&from_value::<String>(refresh_token.await)).await
}

fn generate_verification_code() -> String {
    let mut rng = thread_rng();
    iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(16)
        .collect()
}

pub async fn hash_password(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("admin").await.unwrap() == "1" {
            if let Some(t) = check_password(body["password"]) {
                return message(t);
            }
            return json!({"hash": hash(body["password"])}).to_string();
        }
    }
    json!({}).to_string()
}

pub async fn verify_account(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("verified").await.unwrap() == "0" {
            if session.get("verification_code").await.unwrap() == body["code"] {
                let email = session.get("not_verified_email").await.unwrap();
                if session.get("not_verified_admin").await.unwrap_or_default() == "1" {
                    refresh_admin_session(&mut session, "email", email, None).await;
                } else {
                    refresh_user_session(&mut session, "email", email, "1").await;
                }
                return json!({ "success": true }).to_string();
            }
        }
    }
    json!({"success": false}).to_string()
}

pub async fn send_email(body: HashMap<&str, &str>) -> String {
    if let Some(mut session) = Session::from_id(body["session"]).await {
        if session.get("admin").await.unwrap() == "1" {
            let mut emails = vec![];
            if body["recipients"] == "all_users" {
                for row in get_some("users", "email").await {
                    emails.push(from_value::<String>(row[0].clone()));
                }
                for row in get_some("admin", "email").await {
                    emails.push(from_value::<String>(row[0].clone()));
                }
            } else {
                emails.push(body["recipient"].to_string());
            }
            gmail::send_email(
                emails,
                body["subject"],
                body["body"],
                get_access_token().await.as_str(),
            ).await;
            return json!({ "success": true }).to_string();
        }
    }
    json!({ "success": false }).to_string()
}
