use mysql::{params, Params, Value};

use crate::get_mysql_conn;

pub fn get_like(table: &str, column_name: &str, column_value: &str) -> Vec<Vec<Value>> {
    mysql_statement(
        format!("SELECT * FROM {} WHERE {} LIKE :value", table, column_name),
        params!("value" => column_value),
    )
}

pub fn get_all_rows(table: &str) -> Vec<Vec<Value>> {
    mysql_statement(format!("SELECT * FROM {}", table), ())
}

pub fn mysql_statement<T: Into<Params>>(request: String, params: T) -> Vec<Vec<Value>> {
    let mut conn = get_mysql_conn();
    conn.prep_exec(request, params)
        .unwrap()
        .map(|row| row.unwrap().unwrap())
        .collect()
}
