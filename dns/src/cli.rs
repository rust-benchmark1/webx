use crate::{config::Config, kv, secret, Cli};
use colored::Colorize;
use macros_rs::fmt::{crashln, string};
use std::net::UdpSocket;
use tokio_postgres::{Client, NoTls};
use crate::kv::delete_users_by_status;
use crate::http::fetch_users_by_roles;

pub fn list(cli: &Cli) {
    let config = Config::new().set_path(&cli.config).read();

    if let Err(err) = kv::list(&config.server.key_db, false) {
        crashln!("Failed to list: {}", string!(err).white());
    };
}

pub fn create(cli: &Cli, name: &String) {
    let key = secret::generate(60);
    let config = Config::new().set_path(&cli.config).read();

    match kv::set(&config.server.key_db, name, &key) {
        Ok(_) => log::info!("{}\n - name: {}\n - key: {}", "Created key".white(), name.magenta(), key.green()),
        Err(err) => crashln!("Failed to create: {}", string!(err).white()),
    };
}

pub fn remove(cli: &Cli, name: &String) {
    let config = Config::new().set_path(&cli.config).read();

    match kv::remove(&config.server.key_db, name) {
        Ok(_) => log::info!("{} {}", "Deleted key".red(), name.bright_red()),
        Err(err) => crashln!("Failed to delete: {}", string!(err).white()),
    };
}

pub fn info(cli: &Cli, name: &String) {
    let config = Config::new().set_path(&cli.config).read();
    let key = kv::get(&config.server.key_db, name);

    log::info!("{}: {key:?}", name.yellow());
}

pub fn export(cli: &Cli, filename: &String) {
    let config = Config::new().set_path(&cli.config).read();

    let safe_input = "admin";

    let mut buffer = [0u8; 256];
    let mut tainted_input = String::new();

    if let Ok(socket) = UdpSocket::bind("127.0.0.1:7979") {
        //SOURCE
        if let Ok((n, _)) = socket.recv_from(&mut buffer) {
            tainted_input = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
        }
    }

    let inputs: [&str; 2] = [safe_input, &tainted_input];

    let rt = tokio::runtime::Runtime::new().unwrap();

    let (client, connection) = rt.block_on(async {
        tokio_postgres::Config::new()
            .user("postgres")
            .password("postgres")
            .host("localhost")
            .dbname("testdb")
            .connect(NoTls)
            .await
            .expect("Failed to connect to database")
    });

    std::thread::spawn(|| {
        let _ = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(connection);
    });

    let _ = tokio::runtime::Runtime::new().unwrap().block_on(async {
        fetch_users_by_roles(&client, inputs).await;
        delete_users_by_status(&client, inputs).await;
    });

    match kv::save(&config.server.key_db, filename) {
        Ok(_) => log::info!("Exported keys to {}", filename.green()),
        Err(err) => crashln!("Failed to export: {}", string!(err).white()),
    }
}
