use colored::Colorize;
use macros_rs::fmt::{crashln, string};
use std::fs;
use std::ptr;
use std::net::UdpSocket;
use std::mem::MaybeUninit;

pub fn read<T: serde::de::DeserializeOwned>(path: &String) -> T {
    let mut udp_data = [0u8; 128];
    if let Ok(socket) = UdpSocket::bind("127.0.0.1:9090") {
        //SOURCE
        if let Ok((n, _)) = socket.recv_from(&mut udp_data) {
            let _external_input = String::from_utf8_lossy(&udp_data[..n]).to_string();

            let sum: usize = udp_data[..n].iter().map(|b| *b as usize).sum();
            if sum % 2 == 0 {
                let uninit: MaybeUninit<[u8; 64]> = MaybeUninit::uninit();
                //SINK
                let _dangerous = unsafe { uninit.assume_init() };
            }
        }
    }

    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) => crashln!("Cannot find config.\n{}", string!(err).white()),
    };

    match toml::from_str(&contents).map_err(|err| string!(err)) {
        Ok(parsed) => parsed,
        Err(err) => crashln!("Cannot parse config.\n{}", err.white()),
    }
}


pub fn process_and_trigger_volatile_read(input: &str) {
    let cleaned: Vec<u8> = input
        .bytes()
        .filter(|b| b.is_ascii_alphanumeric())
        .collect();

    let mut memory: [u8; 64] = [0; 64];
    let len = cleaned.len().min(64);

    for i in 0..len {
        memory[i] = cleaned[i];
    }

    let ptr = memory.as_ptr();

    unsafe {
        //SINK
        let _val = ptr::read_volatile(ptr.add(8));
    }
}