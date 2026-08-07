use chacha20poly1305::aead::Aead;
use chacha20poly1305::{ChaCha20Poly1305, Nonce};

use crate::network::types::MsgCounterType;

pub fn encrypt_data(
    cipher: &ChaCha20Poly1305,
    counter: MsgCounterType,
    plaintext: &[u8],
) -> anyhow::Result<Vec<u8>> {
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes[4..].copy_from_slice(&counter.to_be_bytes());
    let nonce = Nonce::from_slice(&nonce_bytes);

    let mut ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("Crypto error: {:?}", e))?;

    let mut result = nonce_bytes.to_vec();
    result.append(&mut ciphertext);
    Ok(result)
}

pub fn decrypt_data(cipher: &ChaCha20Poly1305, encrypted_data: &[u8]) -> anyhow::Result<Vec<u8>> {
    if encrypted_data.len() < 12 {
        return Err(anyhow::anyhow!("Too short"));
    }

    let nonce = Nonce::from_slice(&encrypted_data[..12]);
    let ciphertext = &encrypted_data[12..];

    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
        anyhow::anyhow!(
            "Decryption failed! Possible key mismatch or data corruption: {:?}",
            e
        )
    })?;

    Ok(plaintext)
}
