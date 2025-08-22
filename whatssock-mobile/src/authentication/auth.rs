use std::{fs, path::PathBuf};

use anyhow::Result;
use chacha20poly1305::{
    aead::{Aead, OsRng},
    AeadCore, ChaCha20Poly1305, KeyInit,
};
use sha2::{Digest, Sha256};
use whatssock_lib::{server::LoginResponse, UserSession};

pub fn deserialize_into_login_response(json_input: String) -> Result<LoginResponse> {
    Ok(serde_json::from_str(&json_input)?)
}

pub fn deserialize_into_user_session(json_input: String) -> Result<UserSession> {
    Ok(serde_json::from_str(&json_input)?)
}

pub fn store_user_session_on_disk(user_session: &UserSession, path: PathBuf) -> Result<()> {
    let hwid_key = create_hwid_key()?;

    let serialized_bytes = rmp_serde::to_vec(&user_session)?;

    let encrypted_user_session = encrypt_bytes(serialized_bytes, hwid_key)?;

    fs::write(path, encrypted_user_session)?;

    Ok(())
}

pub fn create_hwid_key() -> Result<
    sha2::digest::generic_array::GenericArray<
        u8,
        sha2::digest::typenum::UInt<
            sha2::digest::typenum::UInt<
                sha2::digest::typenum::UInt<
                    sha2::digest::typenum::UInt<
                        sha2::digest::typenum::UInt<
                            sha2::digest::typenum::UInt<
                                sha2::digest::typenum::UTerm,
                                chacha20poly1305::consts::B1,
                            >,
                            chacha20poly1305::consts::B0,
                        >,
                        chacha20poly1305::consts::B0,
                    >,
                    chacha20poly1305::consts::B0,
                >,
                chacha20poly1305::consts::B0,
            >,
            chacha20poly1305::consts::B0,
        >,
    >,
    anyhow::Error,
> {
    // let hwid_key = Sha256::digest(hwid.as_bytes());
    Ok(todo!())
}

pub fn encrypt_bytes(
    serialized_bytes: Vec<u8>,
    key: sha2::digest::generic_array::GenericArray<
        u8,
        sha2::digest::typenum::UInt<
            sha2::digest::typenum::UInt<
                sha2::digest::typenum::UInt<
                    sha2::digest::typenum::UInt<
                        sha2::digest::typenum::UInt<
                            sha2::digest::typenum::UInt<
                                sha2::digest::typenum::UTerm,
                                chacha20poly1305::consts::B1,
                            >,
                            chacha20poly1305::consts::B0,
                        >,
                        chacha20poly1305::consts::B0,
                    >,
                    chacha20poly1305::consts::B0,
                >,
                chacha20poly1305::consts::B0,
            >,
            chacha20poly1305::consts::B0,
        >,
    >,
) -> Result<Vec<u8>, anyhow::Error> {
    let cipher = ChaCha20Poly1305::new(&key);

    // [u8; 12]
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let encrypted_user_session = cipher
        .encrypt(&nonce, serialized_bytes.as_slice())
        .map_err(|err| anyhow::Error::msg(err.to_string()))?;

    let mut stored_bytes = nonce.to_vec();

    stored_bytes.extend(encrypted_user_session);

    Ok(stored_bytes)
}

pub fn decrypt_bytes(
    mut encrypted_bytes: Vec<u8>,
    key: sha2::digest::generic_array::GenericArray<
        u8,
        sha2::digest::typenum::UInt<
            sha2::digest::typenum::UInt<
                sha2::digest::typenum::UInt<
                    sha2::digest::typenum::UInt<
                        sha2::digest::typenum::UInt<
                            sha2::digest::typenum::UInt<
                                sha2::digest::typenum::UTerm,
                                chacha20poly1305::consts::B1,
                            >,
                            chacha20poly1305::consts::B0,
                        >,
                        chacha20poly1305::consts::B0,
                    >,
                    chacha20poly1305::consts::B0,
                >,
                chacha20poly1305::consts::B0,
            >,
            chacha20poly1305::consts::B0,
        >,
    >,
) -> Result<UserSession, anyhow::Error> {
    let nonce = encrypted_bytes.drain(0..12).collect::<Vec<u8>>();

    let cipher = ChaCha20Poly1305::new(&key);

    let nonce = nonce.as_slice().try_into()?;

    let decrypted_bytes = cipher
        .decrypt(nonce, encrypted_bytes.as_slice())
        .map_err(|err| anyhow::Error::msg(err.to_string()))?;

    // Serialize bytes
    Ok(rmp_serde::from_slice::<UserSession>(&decrypted_bytes)?)
}
