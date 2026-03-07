-- 为 ssh_keys 表增加 key_type 字段（ed25519 / rsa / ecdsa-p256 / ecdsa-p384）
ALTER TABLE ssh_keys ADD COLUMN key_type TEXT NOT NULL DEFAULT 'ed25519';
