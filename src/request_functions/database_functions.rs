use mysql::{from_value, params, Conn, OptsBuilder, Params, Value};

pub fn get_like(table: &str, column_name: &str, column_value: &str) -> Vec<Vec<Value>> {
    let checked_table = check_table(table).unwrap();
    mysql_statement(
        format!(
            "SELECT * FROM {} WHERE {} LIKE :value",
            checked_table, column_name
        ),
        params!("value" => column_value),
    )
    .unwrap()
}

pub fn get_some(table: &str, values: &str) -> Vec<Vec<Value>> {
    let checked_table = check_table(table).unwrap();
    mysql_statement(format!("SELECT ({}) FROM {}", values, checked_table), ()).unwrap()
}

pub fn get_all_rows(table: &str) -> Vec<Vec<Value>> {
    let checked_table = check_table(table).unwrap();
    mysql_statement(format!("SELECT * FROM {} ORDER BY id", checked_table), ()).unwrap()
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

pub fn get_column_details(table: &str) -> Vec<Vec<Value>> {
    let checked_table = check_table(table).unwrap();
    mysql_statement(format!("SHOW COLUMNS FROM {}", checked_table), ()).unwrap()
}

pub fn mysql_statement<T: Into<Params>>(
    request: String,
    params: T,
) -> Result<Vec<Vec<Value>>, String> {
    let mut builder = OptsBuilder::new();
    builder
        .db_name(Some("olmmcc"))
        .user(Some("justus"))
        .pass(Some(""));
    let mut conn = Conn::new(builder).unwrap();
    let result = conn.prep_exec(request, params);
    match result {
        Ok(r) => Ok(r.map(|row| row.unwrap().unwrap()).collect()),
        Err(r) => Err(format!("{}", r)),
    }
}

pub fn row_exists(table: &str, column_name: &str, column_value: &str) -> bool {
    let result = get_like(table, column_name, column_value);
    for vec in result {
        for _ in vec {
            return true;
        }
    }
    false
}

pub fn insert_row(table: &str, titles: Vec<&str>, contents: Vec<&str>) -> Result<(), String> {
    let checked_table = check_table(table).unwrap();
    mysql_statement(
        format!(
            "INSERT INTO {} ({}) VALUES ({}?)",
            checked_table,
            titles.join(", "),
            "?,".to_string().repeat(titles.len() - 1)
        ),
        Params::from(contents),
    )?;
    Ok(())
}

pub fn change_row_where(table: &str, where_name: &str, wherevalue: &str, name: &str, value: &str) {
    let checked_table = check_table(table).unwrap();
    mysql_statement(
        format!(
            "UPDATE {} SET {} = :value WHERE {} = :wherevalue",
            checked_table, name, where_name
        ),
        params!(value, wherevalue),
    )
    .unwrap();
}

pub fn get_max_id(table: &str) -> i32 {
    from_value(mysql_statement(format!("SELECT MAX(id) FROM {}", table), ()).unwrap()[0][0].clone())
}

pub fn get_min_id(table: &str) -> i32 {
    from_value(mysql_statement(format!("SELECT MIN(id) FROM {}", table), ()).unwrap()[0][0].clone())
}

pub fn delete_row_where(table: &str, where_name: &str, wherevalue: &str) {
    let checked_table = check_table(table).unwrap();
    mysql_statement(
        format!(
            "DELETE FROM {} WHERE {} = :wherevalue",
            checked_table, where_name
        ),
        params!(wherevalue),
    )
    .unwrap();
}
