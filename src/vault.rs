use crate::error::VaultError;
use argon2::password_hash::rand_core::RngCore;
use argon2::Argon2;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

const META_FILE: &str = "vault.meta.json";
const CHECK_FILE: &str = "vault.check.bin";
const CHECK_PAYLOAD: &[u8] = b"silentnode:vault-check:v1";

#[derive(Debug)]
pub struct Vault {
    root: PathBuf,
    salt: [u8; 16],
    key: Option<Zeroizing<[u8; 32]>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VaultMetadata {
    salt: Vec<u8>,
}

impl Vault {
    pub fn open(password: &str, vault_path: &Path) -> Result<Self, VaultError> {
        fs::create_dir_all(vault_path)?;
        let (salt, initialized) = load_or_create_salt(vault_path)?;
        let key = derive_key(password, &salt)?;
        let vault = Self {
            root: vault_path.to_path_buf(),
            salt,
            key: Some(Zeroizing::new(key)),
        };

        if initialized {
            let payload = vault.encrypt(CHECK_PAYLOAD)?;
            fs::write(vault.check_path(), payload)?;
            Ok(vault)
        } else {
            let ciphertext = fs::read(vault.check_path())?;
            if vault.decrypt(&ciphertext)? != CHECK_PAYLOAD {
                return Err(VaultError::InvalidPassword);
            }
            Ok(vault)
        }
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, VaultError> {
        let key = self.key.as_ref().ok_or(VaultError::Locked)?;
        let cipher = XChaCha20Poly1305::new((&key[..]).into());
        let mut nonce_bytes = [0u8; 24];
        rand_core::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| VaultError::EncryptionFailure)?;
        let mut output = nonce_bytes.to_vec();
        output.extend(ciphertext);
        Ok(output)
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, VaultError> {
        if ciphertext.len() < 24 {
            return Err(VaultError::MalformedCiphertext);
        }

        let key = self.key.as_ref().ok_or(VaultError::Locked)?;
        let cipher = XChaCha20Poly1305::new((&key[..]).into());
        let (nonce_bytes, payload) = ciphertext.split_at(24);
        let nonce = XNonce::from_slice(nonce_bytes);
        cipher
            .decrypt(nonce, payload)
            .map_err(|_| VaultError::DecryptionFailure)
    }

    pub fn change_password(&mut self, new_password: &str) -> Result<(), VaultError> {
        let new_salt = new_random_salt();
        let new_key = derive_key(new_password, &new_salt)?;
        self.salt = new_salt;
        self.key = Some(Zeroizing::new(new_key));
        write_metadata(&self.root, &self.salt)?;
        fs::write(self.check_path(), self.encrypt(CHECK_PAYLOAD)?)?;
        Ok(())
    }

    pub fn lock(&mut self) {
        self.key = None;
    }

    pub fn is_locked(&self) -> bool {
        self.key.is_none()
    }

    fn check_path(&self) -> PathBuf {
        self.root.join(CHECK_FILE)
    }
}

fn load_or_create_salt(path: &Path) -> Result<([u8; 16], bool), VaultError> {
    let meta_path = path.join(META_FILE);
    if meta_path.exists() {
        let metadata: VaultMetadata = serde_json::from_slice(&fs::read(meta_path)?)?;
        let salt: [u8; 16] = metadata
            .salt
            .try_into()
            .map_err(|_| VaultError::InvalidPassword)?;
        Ok((salt, false))
    } else {
        let salt = new_random_salt();
        write_metadata(path, &salt)?;
        Ok((salt, true))
    }
}

fn write_metadata(path: &Path, salt: &[u8; 16]) -> Result<(), VaultError> {
    fs::write(
        path.join(META_FILE),
        serde_json::to_vec_pretty(&VaultMetadata {
            salt: salt.to_vec(),
        })?,
    )?;
    Ok(())
}

fn derive_key(password: &str, salt: &[u8; 16]) -> Result<[u8; 32], VaultError> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|error| VaultError::Argon2(error.to_string()))?;
    Ok(key)
}

fn new_random_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand_core::OsRng.fill_bytes(&mut salt);
    salt
}
