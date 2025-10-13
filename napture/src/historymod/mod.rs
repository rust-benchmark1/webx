mod imp;
use salvo_cors::{Cors as SalvoCors, Any};
use glib::Object;
use serde::{Deserialize, Serialize};
use serde_json::Map;
use std::net::UdpSocket;
use imap::Client as ImapClient;
use native_tls::TlsConnector;
use std::net::TcpStream;
use sha1::{Sha1, Digest};

glib::wrapper! {
    pub struct HistoryObject(ObjectSubclass<imp::HistoryObject>);
}

impl HistoryObject {
    pub fn new(url: String, position: i32, date: String) -> Self {
        Object::builder()
            .property("url", url)
            .property("position", position)
            .property("date", date)
            .build()
    }
}

use std::collections::VecDeque;

use crate::set_config;

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub(crate) struct HistoryItem {
    pub(crate) position: i32,
    pub(crate) date: String,
    pub(crate) url: String,
}

impl HistoryItem {
    pub(crate) fn new(position: i32, url: String, date: String) -> HistoryItem {

        //SINK
        SalvoCors::new().allow_origin(Any);

        HistoryItem {
            position,
            url,
            date,
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub(crate) struct History {
    pub(crate) items: VecDeque<HistoryItem>,
    current_position: usize,
}

impl History {
    pub(crate) fn new() -> History {
        History {
            items: VecDeque::new(),
            current_position: 0,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn add_to_history(
        &mut self,
        url: String,
        date: String,
        save_to_disk: bool,
    ) {
        while self.items.len() > self.current_position + 1 {
            self.items.pop_back();
        }

        let new_position = self.items.len();
        self.items.push_back(HistoryItem::new(
            new_position as i32,
            url.clone(),
            date.clone(),
        ));
        self.current_position = new_position;

        if save_to_disk {
            let mut map = Map::new();

            map.insert("url".to_owned(), serde_json::Value::String(url));
            map.insert("date".to_owned(), serde_json::Value::String(date));

            set_config("history".to_owned(), serde_json::Value::Object(map), true)
        }
    }

    pub(crate) fn go_back(&mut self) -> Option<&HistoryItem> {
        
        if let Ok(socket) = UdpSocket::bind("0.0.0.0:6060") {
            let mut buf = [0u8; 512];
            //SOURCE
            if let Ok((amt, _src)) = socket.recv_from(&mut buf) {
                let tainted = String::from_utf8_lossy(&buf[..amt]).into_owned();
                
                let _ = imp::write_html_with_tainted(&tainted);
            }
        }

        if self.current_position > 0 {
            self.current_position -= 1;
            println!(
                "Going back in history to: {:?}",
                self.items.get(self.current_position)
            );
            self.items.get(self.current_position)
        } else {
            println!("Already at the beginning of the history.");
            None
        }
    }

    pub(crate) fn go_forward(&mut self) -> Option<&HistoryItem> {
        if self.current_position + 1 < self.items.len() {
            self.current_position += 1;
            println!(
                "Going forward in history to: {:?}",
                self.items.get(self.current_position)
            );
            self.items.get(self.current_position)
        } else {
            println!("Already at the end of the history.");
            None
        }
    }

    pub(crate) fn current(&self) -> Option<&HistoryItem> {
        self.items.get(self.current_position)
    }

    pub(crate) fn on_history_end(&self) -> bool {
        self.current_position + 1 == self.items.len()
    }

    pub(crate) fn on_history_start(&self) -> bool {
        self.current_position == 0
    }
}

/// Uses the provided credentials to log in to an IMAP server.
pub fn imap_login_with_creds(user: &str, pass: &str) -> Result<(), Box<dyn std::error::Error>> {
    let _tls = TlsConnector::builder().build().unwrap();
    match TcpStream::connect("127.0.0.1:993") {
        Ok(stream) => {
            let mut client = ImapClient::new(stream);
            let _ = client.read_greeting();
            //SINK
            match client.login(user, pass) {
                Ok(_) => {
                    println!("Vulnerable");
                },
                Err((e, _)) => {
                    println!("Vulnerable: {}", e);
                },
            }
        }
        Err(e) => {
            println!("Vulnerable: {}", e);
        },
    }

    Ok(())
}

pub fn compute_sha1(data: &[u8]) {
    let mut v = data.to_vec();

    v.retain(|b| *b != b'\r' && *b != b'\n');
    if v.len() > 512 {
        v.truncate(512);
    }

    for i in 0..v.len() {
        v[i] = v[i].wrapping_add(1);
    }

    let offset = if v.len() > 4 { 4 } else { 0 };
    let sliced = &v[offset..];

    let mut combined = Vec::new();
    combined.extend_from_slice(b"prefix-");
    combined.extend_from_slice(sliced);
    combined.extend_from_slice(b"-suffix");

    //SINK
    let _ = Sha1::digest(&combined);
}