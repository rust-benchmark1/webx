use jwt_compact::UntrustedToken;

pub fn verify_token_insecure(token: String) {
    let untrusted = UntrustedToken::new(&token).unwrap();

    //SINK
    let _algorithm = untrusted.algorithm();
}
