pub fn decrypt(data: Vec<u8>, priv_key: &rsa::RsaPrivateKey) -> Result<Vec<u8>, rsa::Error> {
    let mut packet: Vec<u8> = Vec::new();
    let chunks = data[..data.len()].chunks(256);
    for chunk in chunks {
        // TODO: bottleneck.
        let mut p = match priv_key.decrypt(rsa::Pkcs1v15Encrypt, &chunk[..chunk.len()]) {
            Ok(e) => e,
            Err(_) => {
                continue;
            }
        };

        packet.append(&mut p);
    }

    return Ok(packet);
}

pub fn encrypt(data: Vec<u8>, pub_key: &rsa::RsaPublicKey) -> Result<Vec<u8>, rsa::Error> {
    let mut buffer: Vec<u8> = Vec::new();
    let chunks = data[..data.len()].chunks(128);
    for chunk in chunks {
        let mut rng = rand::thread_rng();
        let enc = pub_key.encrypt(&mut rng, rsa::Pkcs1v15Encrypt, &chunk[..chunk.len()]);
        if let Err(e) = enc {
            return Err(e);
        }

        buffer.append(&mut enc.unwrap());
    }

    return Ok(buffer);
}
