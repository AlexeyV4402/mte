use std::time::Duration;

use anyhow::Context;
use chacha20poly1305::ChaCha20Poly1305;
use chacha20poly1305::aead::KeyInit;
use rand::rngs::OsRng;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use x25519_dalek::{EphemeralSecret, PublicKey};

use crate::network::crypt::{decrypt_data, encrypt_data};
use crate::network::types::MsgCounterType;

pub const TIMEOUT_SECONDS: u64 = 20;

pub async fn perform_handshake_client(stream: &mut TcpStream) -> anyhow::Result<ChaCha20Poly1305> {
    let my_secret = EphemeralSecret::random_from_rng(OsRng);
    let my_public = PublicKey::from(&my_secret);

    stream.write_all(my_public.as_bytes()).await?;
    let mut other_public_bytes = [0u8; 32];
    stream.read_exact(&mut other_public_bytes).await?;
    let other_public = PublicKey::from(other_public_bytes);
    let shared_secret = my_secret.diffie_hellman(&other_public);
    let key = shared_secret.as_bytes();

    let key_array: &[u8; 32] = key[..32].try_into()?;

    Ok(ChaCha20Poly1305::new(key_array.into()))
}

pub async fn perform_handshake_server(stream: &mut TcpStream) -> anyhow::Result<ChaCha20Poly1305> {
    let my_secret = EphemeralSecret::random_from_rng(OsRng);
    let my_public = PublicKey::from(&my_secret);

    let mut other_public_bytes = [0u8; 32];
    stream.read_exact(&mut other_public_bytes).await?;

    stream.write_all(my_public.as_bytes()).await?;

    let other_public = PublicKey::from(other_public_bytes);
    let shared_secret = my_secret.diffie_hellman(&other_public);
    let key = shared_secret.as_bytes();
    let key_array: &[u8; 32] = key[..32].try_into()?;

    Ok(ChaCha20Poly1305::new(key_array.into()))
}

pub async fn send_tcp(
    stream: &mut TcpStream,
    payload: Vec<u8>,
    msg_counter: &mut MsgCounterType,
    cipher: &ChaCha20Poly1305,
) -> anyhow::Result<()> {
    let encrypted = encrypt_data(cipher, *msg_counter, &payload)?;
    let len = encrypted.len() as u32;

    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&encrypted).await?;

    *msg_counter += 1;
    Ok(())
}

pub async fn read_tcp(
    stream: &mut TcpStream,
    cipher: &ChaCha20Poly1305,
) -> anyhow::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];

    let encrypted_data = timeout(Duration::from_secs(TIMEOUT_SECONDS), async {
        stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        if len > 65536 || len < 12 {
            return Err(anyhow::anyhow!("Недопустимая длина пакета: {}", len));
        }

        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf).await?;
        Ok(buf)
    })
    .await
    .context("Таймаут чтения данных")??;

    decrypt_data(cipher, &encrypted_data).map_err(|e| anyhow::anyhow!("Ошибка дешифровки: {}", e))
}
