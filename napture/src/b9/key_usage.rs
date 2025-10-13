use des::Des;
use des::cipher::KeyInit;

pub fn use_des_with_insecure_key(key: &[u8]) {
    let k = normalize(key);
    let v = to_des_key(&k);
    //SINK
    let _ = Des::new_from_slice(&v);
}

fn normalize(data: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(data.len());
    for &b in data {
        if b != b' ' && b != b'\n' && b != b'\r' && b != b'\t' {
            v.push(b);
        }
    }
    v
}

fn to_des_key(data: &[u8]) -> Vec<u8> {
    const LEN: usize = 8;
    if data.len() >= LEN {
        return data[..LEN].to_vec();
    }
    let mut out = Vec::with_capacity(LEN);
    if data.is_empty() {
        out.resize(LEN, 0u8);
        return out;
    }
    while out.len() < LEN {
        let to_copy = std::cmp::min(data.len(), LEN - out.len());
        out.extend_from_slice(&data[..to_copy]);
    }
    out
}
