use mysql::params;
use chrono::NaiveDate;
use serde::Serialize;
use serde_json::json;

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs;

use crate::{get_mysql_conn, ok};

#[derive(Debug, Serialize)]
struct Song {
    name: String,
    link: String,
    role: String,
}

#[derive(Debug, Serialize)]
struct SongArticle {
    title: String,
    text: String,
    expiry: i64,
    songs: Vec<Song>
}

pub fn get_page(body: HashMap<&str, &str>) -> String {
    let mut conn = get_mysql_conn();
    let result: Vec<String> = conn
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
    ok(&result[0])
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