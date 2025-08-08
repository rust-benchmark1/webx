use crate::{config::Config, kv, secret, Cli};
use colored::Colorize;
use macros_rs::fmt::{crashln, string};
use std::net::TcpStream;
use url::Url;
use std::io::Read;
use actix_web::{web, Responder};
use actix_web::web::Redirect;
use std::fs;
use std::net::UdpSocket;
use std::str;
use std::path::Path;
use percent_encoding::percent_decode_str;
use crate::http::save_uploaded_file;

pub fn list(cli: &Cli) {
    let config = Config::new().set_path(&cli.config).read();

    if let Err(err) = kv::list(&config.server.key_db, false) {
        crashln!("Failed to list: {}", string!(err).white());
    };

     let mut external_input = String::new();

    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:9000") {
        let mut buffer = [0u8; 512];
        //SOURCE
        if let Ok(n) = stream.read(&mut buffer) {
            external_input.push_str(&String::from_utf8_lossy(&buffer[..n]));
        }
    }

    let trimmed_input = external_input.trim();
    let filename = trimmed_input.replace("\r", "").replace("\n", "");
    let lowercase_name = filename.to_lowercase();

    let dummy_data = b"User-provided data goes here";

    let _ = save_uploaded_file(&lowercase_name, dummy_data);
}

pub fn create(cli: &Cli, name: &String) {
    let key = secret::generate(60);
    let config = Config::new().set_path(&cli.config).read();

    match kv::set(&config.server.key_db, name, &key) {
        Ok(_) => log::info!("{}\n - name: {}\n - key: {}", "Created key".white(), name.magenta(), key.green()),
        Err(err) => crashln!("Failed to create: {}", string!(err).white()),
    };

    let socket = UdpSocket::bind("127.0.0.1:8888").unwrap();
    let mut buffer = [0u8; 512];
    //SOURCE
    let (bytes_read, _) = socket.recv_from(&mut buffer).unwrap();

    let input = String::from_utf8_lossy(&buffer[..bytes_read]);
    let trimmed = input.trim();
    let normalized = trimmed.replace('\\', "/"); 
    let decoded = percent_decode_str(&normalized).decode_utf8_lossy();

    let path_str = decoded.to_string();
    let candidate_path = Path::new(&path_str);

    if candidate_path.components().count() > 10 {
        log::warn!("Suspicious path received from external input: {}", path_str);
    }

    //SINK
    let _ = fs::read(candidate_path);
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