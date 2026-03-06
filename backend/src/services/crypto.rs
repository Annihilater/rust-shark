use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;

pub struct CryptoService {
    key: Key<Aes256Gcm>,
}

impl CryptoService {
    pub fn new(secret_key: &str) -> Result<Self> {
        // 确保密钥是32字节
        let mut key_bytes = [0u8; 32];
        let secret_bytes = secret_key.as_bytes();
        let len = secret_bytes.len().min(32);
        key_bytes[..len].copy_from_slice(&secret_bytes[..len]);

        let key = Key::<Aes256Gcm>::from_slice(&key_bytes).clone();
        Ok(Self { key })
    }

    /// 加密数据，返回 base64(nonce + ciphertext)
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let cipher = Aes256Gcm::new(&self.key);
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| anyhow::anyhow!("加密失败: {}", e))?;

        // 组合 nonce + ciphertext 然后 base64 编码
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&combined))
    }

    /// 解密 base64(nonce + ciphertext)
    pub fn decrypt(&self, encrypted: &str) -> Result<String> {
        let combined = BASE64
            .decode(encrypted)
            .context("base64解码失败")?;

        if combined.len() < 12 {
            anyhow::bail!("密文太短");
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        let cipher = Aes256Gcm::new(&self.key);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("解密失败: {}", e))?;

        String::from_utf8(plaintext).context("UTF8解码失败")
    }
}
