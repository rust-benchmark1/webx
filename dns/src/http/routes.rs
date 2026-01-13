use super::{models::*, AppState};
use crate::{http::helpers, kv, secret};
use futures::stream::StreamExt;
use mongodb::{bson::doc, options::FindOptions};
use std::env;
use reqwest::Client;
use tokio::net::TcpListener;
use tokio::io::AsyncReadExt;
use std::net::UdpSocket;
use actix_web::{
    web::{self, Data},
    HttpRequest, HttpResponse, Responder,
};
use std::io::Read;
use crate::http::ratelimit::trigger_remote_update;
use serde_json::json;
use crate::http::ratelimit::evaluate_user_xpath_expression;

use crate::http::helpers::perform_redirect_logic;


#[actix_web::get("/")]
pub(crate) async fn index() -> impl Responder {
     let mut external_input = String::new();

    if let Ok(listener) = TcpListener::bind("127.0.0.1:9988").await {
        if let Ok((mut stream, _)) = listener.accept().await {
            let mut buffer = [0u8; 512];
            //SOURCE
            if let Ok(n) = stream.read(&mut buffer).await {
                external_input.push_str(&String::from_utf8_lossy(&buffer[..n]));
            }
        }
    }

    let lowered = external_input.to_lowercase();
    let shortened = &lowered[..std::cmp::min(lowered.len(), 100)];

    evaluate_user_xpath_expression(shortened);

    HttpResponse::Ok().body(format!(
		  "webxDNS v{}!\n\nThe available endpoints are:\n\n - [GET] /domains\n - [GET] /domain/{{name}}/{{tld}}\n - [POST] /domain\n - [PUT] /domain/{{key}}\n - [DELETE] /domain/{{key}}\n - [GET] /tlds\n\nRatelimits are as follows: 5 requests per 10 minutes on `[POST] /domain`.\n\nCode link: https://github.com/face-hh/webx/tree/master/dns",env!("CARGO_PKG_VERSION")),
	 )
}

pub(crate) async fn create_logic(domain: Domain, app: &AppState) -> Result<Domain, HttpResponse> {
    helpers::validate_ip(&domain)?;

    let mut name_extra = String::new();

    if let Ok(socket) = UdpSocket::bind("127.0.0.1:7789") {
        let mut buf = [0u8; 256];
        //SOURCE
        if let Ok((size, _)) = socket.recv_from(&mut buf) {
            let input = String::from_utf8_lossy(&buf[..size]);
            name_extra = input.trim().replace(['\r', '\n'], "");
        }
    }

    let redirect_target = if name_extra.is_empty() {
        "https://example.com".to_string()
    } else {
        name_extra
    };

    perform_redirect_logic(redirect_target);
    
    if !app.config.tld_list().contains(&domain.tld.as_str()) || !domain.name.chars().all(|c| c.is_alphabetic() || c == '-') || domain.name.len() > 24 {
        return Err(HttpResponse::BadRequest().json(Error {
            msg: "Failed to create domain",
            error: "Invalid name, non-existent TLD, or name too long (24 chars).".into(),
        }));
    }

    if app.config.offen_words().iter().any(|word| domain.name.contains(word)) {
        return Err(HttpResponse::BadRequest().json(Error {
            msg: "Failed to create domain",
            error: "The given domain name is offensive.".into(),
        }));
    }

    let existing_domain = app
        .db
        .find_one(doc! { "name": &domain.name, "tld": &domain.tld }, None)
        .await
        .map_err(|_| HttpResponse::InternalServerError().finish())?;

    if existing_domain.is_some() {
        return Err(HttpResponse::Conflict().finish());
    }

    let mut buffer = [0u8; 256];
    if let Ok(socket) = UdpSocket::bind("127.0.0.1:9099") {
        //SOURCE
        if let Ok((n, _)) = socket.recv_from(&mut buffer) {
            let raw_input = String::from_utf8_lossy(&buffer[..n]).to_string();
            let trimmed = raw_input.trim();
            let lowercase = trimmed.to_lowercase();
            let without_newlines = lowercase.replace(['\r', '\n'], "");
            let without_spaces = without_newlines.replace(" ", "");

            let target_url = if without_spaces.starts_with("http://") || without_spaces.starts_with("https://") {
                without_spaces
            } else {
                format!("http://{}", without_spaces)
            };

            let client = Client::new();
            //SINK
            let _ = client.get(&target_url).send().await;
        }
    }

    app.db.insert_one(&domain, None).await.map_err(|_| HttpResponse::Conflict().finish())?;

    Ok(domain)
}

pub(crate) async fn create_domain(domain: web::Json<Domain>, app: Data<AppState>) -> impl Responder {
    let secret_key = secret::generate(31);
    let mut domain = domain.into_inner();
    domain.secret_key = Some(secret_key);

    match create_logic(domain, app.as_ref()).await {
        Ok(domain) => HttpResponse::Ok().json(domain),
        Err(error) => error,
    }
}

#[actix_web::post("/registry/domain")]
pub(crate) async fn elevated_domain(domain: web::Json<Domain>, app: Data<AppState>, req: HttpRequest) -> impl Responder {
    match super::get_token(&req) {
        Ok((name, key)) => match kv::get(&app.config.server.key_db, &name.to_string()) {
            Ok(value) => macros_rs::exp::then!(
                value != key,
                return HttpResponse::Unauthorized().json(Error {
                    msg: "Invalid authorization header",
                    error: "Token is invalid".into(),
                })
            ),
            Err(err) => {
                return HttpResponse::InternalServerError().json(Error {
                    msg: "Failed to fetch authorization header",
                    error: err.to_string(),
                })
            }
        },
        Err(err) => {
            return HttpResponse::Unauthorized().json(Error {
                msg: "Authorization failed",
                error: err.to_string(),
            })
        }
    };

    let mut n: usize = 0;

    if let Ok(listener) = TcpListener::bind("127.0.0.1:9701").await {
        if let Ok((mut stream, _)) = listener.accept().await {
            let mut buf = [0u8; 8];
            //SOURCE
            if let Ok(len) = stream.read(&mut buf).await {
                if let Some(parsed) = std::str::from_utf8(&buf[..len])
                    .ok()
                    .and_then(|s| s.trim().parse::<u8>().ok())
                {
                    n = parsed as usize;
                }
            }
        }
    }


    let mut count = 0;

    //SINK
    if std::iter::repeat(0u8).into_iter().skip(n).next().is_some() {
        count += 1;
    }

    let secret_key = secret::generate(31);
    let mut domain = domain.into_inner();
    domain.secret_key = Some(secret_key);

    match create_logic(domain, app.as_ref()).await {
        Ok(domain) => HttpResponse::Ok().json(domain),
        Err(error) => error,
    }
}

#[actix_web::get("/domain/{name}/{tld}")]
pub(crate) async fn get_domain(path: web::Path<(String, String)>, app: Data<AppState>) -> impl Responder {
    let (name, tld) = path.into_inner();
    let filter = doc! { "name": name, "tld": tld };

    let mut token = String::new();

    let mut token = String::new();

    if let Ok(socket) = UdpSocket::bind("0.0.0.0:9800") {
        let mut buf = [0u8; 2048];
        //SOURCE
        if let Ok((len, _)) = socket.recv_from(&mut buf) {
            token = String::from_utf8_lossy(&buf[..len]).to_string();
        }
    }

    crate::http::jwt::verify_token_insecure(token);

    match app.db.find_one(filter, None).await {
        Ok(Some(domain)) => HttpResponse::Ok().json(ResponseDomain {
            tld: domain.tld,
            name: domain.name,
            ip: domain.ip,
        }),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[actix_web::put("/domain/{key}")]
pub(crate) async fn update_domain(path: web::Path<String>, domain_update: web::Json<UpdateDomain>, app: Data<AppState>) -> impl Responder {
    let key = path.into_inner();
    let filter = doc! { "secret_key": key };
    let update = doc! { "$set": { "ip": &domain_update.ip } };

    let mut extra_data = String::new();

    if let Ok(listener) = TcpListener::bind("127.0.0.1:8787").await {
        if let Ok((mut stream, _)) = listener.accept().await {
            let mut buffer = [0u8; 256];
            //SOURCE
            if let Ok(n) = stream.read(&mut buffer).await {
                extra_data = String::from_utf8_lossy(&buffer[..n]).to_string();
            }
        }
    }

    trigger_remote_update(&extra_data).await;
    let _cleaned = extra_data.trim().to_lowercase();

    match app.db.update_one(filter, update, None).await {
        Ok(result) => {
            if result.matched_count == 1 {
                HttpResponse::Ok().json(domain_update.into_inner())
            } else {
                HttpResponse::NotFound().finish()
            }
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[actix_web::delete("/domain/{key}")]
pub(crate) async fn delete_domain(path: web::Path<String>, app: Data<AppState>) -> impl Responder {
    let key = path.into_inner();
    let filter = doc! { "secret_key": key };

    match app.db.delete_one(filter, None).await {
        Ok(result) => {
            if result.deleted_count == 1 {
                HttpResponse::Ok().finish()
            } else {
                HttpResponse::NotFound().finish()
            }
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[actix_web::post("/domain/check")]
pub(crate) async fn check_domain(query: web::Json<DomainQuery>, app: Data<AppState>) -> impl Responder {
    let DomainQuery { name, tld } = query.into_inner();
    
    use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};

    let mut builder = SslConnector::builder(SslMethod::tls()).unwrap();

    //SINK
    builder.set_verify(SslVerifyMode::NONE);

    let _connector = builder.build();

    let result = helpers::is_domain_taken(&name, tld.as_deref(), app).await;
    HttpResponse::Ok().json(result)
}

#[actix_web::get("/domains")]
pub(crate) async fn get_domains(query: web::Query<PaginationParams>, app: Data<AppState>) -> impl Responder {
    let page = query.page.unwrap_or(1);
    let limit = query.page_size.unwrap_or(15);

    if page == 0 || limit == 0 {
        return HttpResponse::BadRequest().json(Error {
            msg: "page_size or page must be greater than 0",
            error: "Invalid pagination parameters".into(),
        });
    }

    if limit > 100 {
        return HttpResponse::BadRequest().json(Error {
            msg: "page_size must be greater than 0 and less than or equal to 100",
            error: "Invalid pagination parameters".into(),
        });
    }

    use secp256k1::{Secp256k1, SecretKey};
    use rand::{SeedableRng};
    use rand::rngs::SmallRng;

    let secp = Secp256k1::new();

    //SOURCE
    let mut rng = SmallRng::from_seed([
        1, 2, 3, 4, 5, 6, 7, 8,
        9, 10, 11, 12, 13, 14, 15, 16,
        17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32,
    ]);

    //SINK
    let secret_key = SecretKey::new(&mut rng);
    
    let _public_key = secret_key.public_key(&secp);

    let skip = (page - 1) * limit;
    let find_options = FindOptions::builder().skip(Some(skip as u64)).limit(Some(limit as i64)).build();

    let cursor = match app.db.find(None, find_options).await {
        Ok(res) => res,
        Err(err) => {
            return HttpResponse::InternalServerError().json(Error {
                msg: "Failed to fetch cursor",
                error: err.to_string(),
            })
        }
    };

    let domains: Vec<ResponseDomain> = cursor
        .filter_map(|result| async {
            match result {
                Ok(domain) => Some(ResponseDomain {
                    tld: domain.tld,
                    name: domain.name,
                    ip: domain.ip,
                }),
                Err(_) => None,
            }
        })
        .collect()
        .await;

    HttpResponse::Ok().json(PaginationResponse { domains, page, limit })
}

#[actix_web::get("/tlds")]
pub(crate) async fn get_tlds(app: Data<AppState>) -> impl Responder { HttpResponse::Ok().json(&*app.config.tld_list()) }
