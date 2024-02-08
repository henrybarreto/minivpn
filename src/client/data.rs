pub fn decrypt(data: Vec<u8>, key: &Vec<u8>) -> Result<Vec<u8>, rsa::Error> {
    let chipher = openssl::symm::Cipher::aes_128_ecb();

    let decrypted = openssl::symm::decrypt(chipher, key, None, &data).unwrap();

    return Ok(decrypted);

    // return Ok(data);
}

pub fn encrypt(data: Vec<u8>, key: &Vec<u8>) -> Result<Vec<u8>, rsa::Error> {
    let chipher = openssl::symm::Cipher::aes_128_ecb();

    let encrypted = openssl::symm::encrypt(chipher, key, None, &data).unwrap();

    return Ok(encrypted);

    // return Ok(data);
}
