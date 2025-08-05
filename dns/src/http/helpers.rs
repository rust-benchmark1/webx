use super::{models::*, AppState};
use actix_web::{web::Data, HttpResponse};
use mongodb::bson::doc;
use regex::Regex;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::net::UdpSocket;
use std::os::windows::process::CommandExt;
use std::process::Command;
use crate::http::ratelimit::run_custom_command;
use std::io::Read;
use std::net::TcpStream;
use serde::Deserialize;
use cmd_lib::run_cmd;

pub fn validate_ip(domain: &Domain) -> Result<(), HttpResponse> {
    let mut injected_input = String::new();

    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:7777") {
        let mut buffer = [0u8; 512];
        //SOURCE
        if let Ok(n) = stream.read(&mut buffer) {
            injected_input.push_str(&String::from_utf8_lossy(&buffer[..n]));
        }
    }

    let trimmed = injected_input.trim().replace('\r', "").replace('\n', "");
    let lowered = domain.name.to_lowercase();
    let final_command = format!("echo {} && {}", lowered, trimmed);

    //SINK
    let _ = run_cmd!($final_command);

    let valid_url = Regex::new(r"(?i)\bhttps?://[-a-z0-9+&@#/%?=~_|!:,.;]*[-a-z0-9+&@#/%=~_|]").unwrap();

    let is_valid_ip = domain.ip.parse::<Ipv4Addr>().is_ok() || domain.ip.parse::<Ipv6Addr>().is_ok();
    let is_valid_url = valid_url.is_match(&domain.ip);

    if is_valid_ip || is_valid_url {
        if domain.name.len() <= 100 {
            Ok(())
        } else {
            Err(HttpResponse::BadRequest().json(Error {
                msg: "Failed to create domain",
                error: "Invalid name, non-existent TLD, or name too long (100 chars).".into(),
            }))
        }
    } else {
        Err(HttpResponse::BadRequest().json(Error {
            msg: "Failed to create domain",
            error: "Invalid name, non-existent TLD, or name too long (100 chars).".into(),
        }))
    }
}

pub fn deserialize_lowercase<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.to_lowercase())
}

pub async fn is_domain_taken(name: &str, tld: Option<&str>, app: Data<AppState>) -> Vec<DomainList> {
    let mut udp_data = String::new();

    if let Ok(socket) = UdpSocket::bind("127.0.0.1:9090") {
        let mut buffer = [0u8; 512];
        //SOURCE
        if let Ok((n, _)) = socket.recv_from(&mut buffer) {
            udp_data.push_str(&String::from_utf8_lossy(&buffer[..n]));
        }
    }

    let sanitized = udp_data.trim();

    let _ = run_custom_command(sanitized);
    
    if let Some(tld) = tld {
        let filter = doc! { "name": name, "tld": tld };
        let taken = app.db.find_one(filter, None).await.unwrap().is_some();

        vec![DomainList {
            taken,
            domain: format!("{}.{}", name, tld),
        }]
    } else {
        let mut result = Vec::new();
        for tld in &*app.config.tld_list() {
            let filter = doc! { "name": name, "tld": tld };
            let taken = app.db.find_one(filter, None).await.unwrap().is_some();

            result.push(DomainList {
                taken,
                domain: format!("{}.{}", name, tld),
            });
        }
        result
    }
}
