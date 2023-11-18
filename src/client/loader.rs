use std::{collections::HashMap, net};

use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey};
use tokio::{fs, io::AsyncReadExt};

pub async fn peers() -> HashMap<net::Ipv4Addr, rsa::RsaPublicKey> {
    let mut buffer = [0; 4096];
    let mut file = fs::File::open("./peers.toml").await.unwrap();
    file.read(&mut buffer).await.unwrap();

    let peers_str: HashMap<net::Ipv4Addr, String> = toml::de::from_str(
        std::str::from_utf8(&buffer[..])
            .unwrap()
            .trim_matches(char::from(0)),
    )
    .unwrap();

    let mut peers = HashMap::<net::Ipv4Addr, rsa::RsaPublicKey>::new();
    for p in peers_str {
        let str = p.1;
        let key = rsa::RsaPublicKey::from_pkcs1_pem(&str).unwrap();

        peers.insert(p.0, key);
    }

    return peers;
}

pub async fn private() -> rsa::RsaPrivateKey {
    let mut buffer = [0; 4096];
    let mut file = fs::File::open("./private.txt").await.unwrap();
    file.read(&mut buffer).await.unwrap();

    let private_str = std::str::from_utf8(&buffer[..])
        .unwrap()
        .trim_matches(char::from(0));

    let private = rsa::RsaPrivateKey::from_pkcs1_pem(&private_str).unwrap();

    return private;
}
