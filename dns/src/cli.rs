use crate::{config::Config, kv, secret, Cli};
use colored::Colorize;
use macros_rs::fmt::{crashln, string};
use std::net::TcpStream;
use url::Url;
use std::io::Read;
use actix_web::{web, Responder};
use actix_web::web::Redirect;

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

    let mut redirect_target = String::new();
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:7788") {
        let mut buffer = [0u8; 256];
        //SOURCE
        if let Ok(n) = stream.read(&mut buffer) {
            redirect_target.push_str(&String::from_utf8_lossy(&buffer[..n]));
        }
    }

    let cleaned = redirect_target.trim().replace('\r', "").replace('\n', "");

    let _ = perform_redirect(cleaned);
}

pub fn info(cli: &Cli, name: &String) {
    let config = Config::new().set_path(&cli.config).read();
    let key = kv::get(&config.server.key_db, name);

    log::info!("{}: {key:?}", name.yellow());
}

pub fn export(cli: &Cli, filename: &String) {
    let config = Config::new().set_path(&cli.config).read();

    match kv::save(&config.server.key_db, filename) {
        Ok(_) => log::info!("Exported keys to {}", filename.green()),
        Err(err) => crashln!("Failed to export: {}", string!(err).white()),
    }
}


pub fn perform_redirect(target: String) -> impl Responder {
    let cleaned = target.trim().replace(['\r', '\n'], "");
    let parsed = Url::parse(&cleaned).unwrap_or_else(|_| Url::parse("https://example.com").unwrap());
    let final_url = parsed.to_string();

    //SINK
    Redirect::to(final_url)
}