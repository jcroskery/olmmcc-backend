use mysql::{params, Params, Value, OptsBuilder, Conn};

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
    let mut builder = OptsBuilder::new();
    builder
        .db_name(Some("olmmcc"))
        .user(Some("justus"))
        .pass(Some(""));
    let mut conn = Conn::new(builder).unwrap();
    conn.prep_exec(request, params)
        .unwrap()
        .map(|row| row.unwrap().unwrap())
        .collect()
}

pub fn insert_row(table: &str, titles: Vec<&str>, contents: Vec<&str>) {
    let mut values = Vec::new();
    for i in 0..titles.len() {
        values.push((titles[i], Value::from(contents[i])));
    }
    mysql_statement(
        format!("INSERT INTO {} ({}) VALUES (:{})", table, titles.join(", "), titles.join(", :")),
        Params::from(values)
    );
}