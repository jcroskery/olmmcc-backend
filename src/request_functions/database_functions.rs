use mysql::{params, Value};

use crate::get_mysql_conn;

pub fn get_row(table: &str, column_name: &str, column_value: &str) -> Vec<Vec<Value>> {
    let mut conn = get_mysql_conn();
    conn.prep_exec(format!("SELECT * FROM {} WHERE {} = :value", table, column_name), 
        params!("value" => column_value)).unwrap().map(|row|{row.unwrap().unwrap()})
        .collect()
}