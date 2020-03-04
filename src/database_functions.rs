use mysql_async::{from_value, params, Conn, Params, Value};
use mysql_async::prelude::Queryable;

const URL: &str = "mysql://justus:@localhost:3306/olmmcc";

pub async fn get_like(table: &str, column_name: &str, column_value: &str) -> Vec<Vec<Value>> {
    let checked_table = check_table(table).unwrap();
    let query = format!(
            "SELECT * FROM {} WHERE {} LIKE :value",
            checked_table, column_name
        );
    mysql_statement(
        query,
        params!("value" => column_value),
    ).await
    .unwrap()
}

pub async fn get_some(table: &str, values: &str) -> Vec<Vec<Value>> {
    let checked_table = check_table(table).unwrap();
    let query = format!("SELECT ({}) FROM {}", values, checked_table);
    mysql_statement(query, ()).await.unwrap()
}

pub async fn get_all_rows(table: &str, order: bool) -> Vec<Vec<Value>> {
    let checked_table = check_table(table).unwrap();
    let order = if order { " ORDER BY id" } else { "" };
    let query = format!("SELECT * FROM {}{}", checked_table, order);
    mysql_statement(query, ()).await.unwrap()
}

fn check_table(table: &str) -> Option<&str> {
    const ALLOWED_TABLES: &[&str] = &["admin", "pages", "articles", "calendar", "songs", "users"];
    for allowed_table in ALLOWED_TABLES {
        if *allowed_table == table {
            return Some(allowed_table);
        }
    }
    None
}

pub async fn get_column_details(table: &str) -> Vec<Vec<Value>> {
    let checked_table = check_table(table).unwrap();
    let query = format!("SHOW COLUMNS FROM {}", checked_table);
    mysql_statement(query, ()).await.unwrap()
}

pub async fn mysql_statement<T: Into<Params>>(
    request: String,
    params: T,
) -> Result<Vec<Vec<Value>>, String> {
    let conn = Conn::new(URL).await.unwrap();
    let result = conn.prep_exec(request, params).await;
    match result {
        Ok(r) => Ok(r.map(|row| row.unwrap()).await.unwrap().1),
        Err(r) => Err(format!("{}", r)),
    }
}

pub async fn row_exists(table: &str, column_name: &str, column_value: &str) -> bool {
    let result = get_like(table, column_name, column_value).await;
    for vec in result {
        for _ in vec {
            return true;
        }
    }
    false
}

pub async fn insert_row(table: &str, titles: Vec<&str>, contents: Vec<&str>) -> Result<(), String> {
    let checked_table = check_table(table).unwrap();
    let query = format!(
            "INSERT INTO {} ({}) VALUES ({}?)",
            checked_table,
            titles.join(", "),
            "?,".to_string().repeat(titles.len() - 1)
        );
    mysql_statement(
        query,
        Params::from(contents),
    ).await?;
    Ok(())
}

pub async fn change_row_where(table: &str, where_name: &str, wherevalue: &str, name: &str, value: &str) {
    let checked_table = check_table(table).unwrap();
    mysql_statement(
        format!(
            "UPDATE {} SET {} = :value WHERE {} = :wherevalue",
            checked_table, name, where_name
        ),
        params!(value, wherevalue),
    ).await
    .unwrap();
}

pub async fn get_max_id(table: &str) -> i32 {
    from_value(mysql_statement(format!("SELECT MAX(id) FROM {}", table), ()).await.unwrap()[0][0].clone())
}

pub async fn get_min_id(table: &str) -> i32 {
    from_value(mysql_statement(format!("SELECT MIN(id) FROM {}", table), ()).await.unwrap()[0][0].clone())
}

pub async fn delete_row_where(table: &str, where_name: &str, wherevalue: &str) {
    let checked_table = check_table(table).unwrap();
    mysql_statement(
        format!(
            "DELETE FROM {} WHERE {} = :wherevalue",
            checked_table, where_name
        ),
        params!(wherevalue),
    ).await
    .unwrap();
}
