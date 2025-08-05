use anyhow::{anyhow, Error};
use prettytable::{format, row, Table};
use std::{collections::HashMap, fs::File, str::from_utf8};
use tokio_postgres::Client;

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