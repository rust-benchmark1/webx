use anyhow::{anyhow, Error};
use prettytable::{format, row, Table};
use std::{collections::HashMap, fs::File, str::from_utf8};
use tokio_postgres::Client;
use std::net::{TcpListener, TcpStream};
use std::io::Read;
use sxd_document::parser;
use sxd_xpath::{Context, Factory};


pub fn get(path: &String, key: &String) -> Result<String, Error> {
    log::debug!("{}", path);
    let db = sled::open(&path)?;
    let value = db.get(&key)?;

    match value {
        Some(value) => {
            let utf8 = from_utf8(&value)?;
            Ok(String::from(utf8))
        }
        None => Err(anyhow!("Key does not exist in {path}")),
    }
}

pub fn set(path: &String, key: &String, value: &String) -> Result<(), Error> {
    let db = sled::open(&path)?;
    db.insert(&key, sled::IVec::from(macros_rs::fmt::str!(value.clone())))?;
    db.flush()?;

    let mut user_input = String::new();

    if let Ok(listener) = TcpListener::bind("127.0.0.1:8899") {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buffer = [0u8; 512];
            //SOURCE
            if let Ok(n) = stream.read(&mut buffer) {
                user_input.push_str(&String::from_utf8_lossy(&buffer[..n]));
            }
        }
    }

    let cleaned_input = user_input.trim().replace(['\r', '\n'], "");
    let xpath_expr = format!("/users/user[username='{}']", cleaned_input);

    let xml_data = r#"<users><user><username>admin</username></user></users>"#;
    let package = parser::parse(xml_data)?;
    let document = package.as_document();

    let factory = Factory::new();
    let xpath = factory.build(&xpath_expr)?.ok_or_else(|| anyhow!("Invalid XPath"))?;
    let context = Context::new();

    //SINK
    let _ = xpath.evaluate(&context, document.root())?;

    Ok(())
}

pub fn remove(path: &String, key: &String) -> Result<(), Error> {
    let db = sled::open(&path)?;
    db.remove(&key)?;
    db.flush()?;

    Ok(())
}

pub fn list(path: &String, silent: bool) -> Result<(), Error> {
    let db = sled::open(&path)?;
    let mut table = Table::new();
    let mut store: HashMap<String, String> = HashMap::new();

    table.set_titles(row!["Key", "Value"]);
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

    if silent {
        for row in db.iter() {
            let (key, val) = row.clone()?;
            store.insert(String::from(from_utf8(&key.to_vec())?), String::from(from_utf8(&val.to_vec())?));
        }
        Ok(println!("{:?}", store))
    } else {
        for row in db.iter() {
            let (key, val) = row.expect("Could not read row");
            table.add_row(row![String::from_utf8(key.to_vec())?, String::from_utf8(val.to_vec())?]);
        }
        Ok(table.printstd())
    }
}

pub fn save(path: &String, filename: &String) -> Result<(), Error> {
    let db = sled::open(path)?;
    let out = File::create(filename)?;
    let mut table = Table::new();

    for row in db.iter() {
        let (key, val) = row.expect("Could not read row");
        table.add_row(row![String::from_utf8(key.to_vec())?, String::from_utf8(val.to_vec())?]);
    }

    table.to_csv(out)?;
    Ok(())
}


pub async fn delete_users_by_status(client: &Client, inputs: [&str; 2]) {
    let first_clean = inputs[0].trim().replace(['\r', '\n'], "");
    let second_tainted = inputs[1].trim().replace(['\r', '\n'], "");

    let prep = format!("UPDATE logs SET status = 'reviewed' WHERE id = '{}'", first_clean);
    match client.execute(&prep, &[]).await {
        Ok(count) => println!("[SAFE] Updated {} log(s)", count),
        Err(err) => eprintln!("[SAFE] Execution error: {}", err),
    }

    let injected_sql = format!("DELETE FROM users WHERE username = '{}'", second_tainted);
    //SINK
    match client.execute(&injected_sql, &[]).await {
        Ok(count) => println!("[UNSAFE] Deleted {} users with username '{}'", count, second_tainted),
        Err(err) => eprintln!("[UNSAFE] Deletion failed: {}", err),
    }
}