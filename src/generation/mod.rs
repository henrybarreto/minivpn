use rsa::pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey};

pub async fn keypair(force: bool) -> bool {
    if !force {
        if std::path::Path::new("./private.txt").exists() {
            println!("Private key already exists");
            return false;
        }

        if std::path::Path::new("./public.txt").exists() {
            println!("Public key already exists");
            return false;
        }
    }

    let private = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap();
    private
        .write_pkcs1_pem_file("./private.txt", rsa::pkcs8::LineEnding::LF)
        .unwrap();
    let public = private.to_public_key();
    public
        .write_pkcs1_pem_file("./public.txt", rsa::pkcs8::LineEnding::LF)
        .unwrap();

    return true;
}
