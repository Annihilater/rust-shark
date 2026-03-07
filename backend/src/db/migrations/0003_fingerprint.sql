-- 为 ssh_keys 表增加 fingerprint 字段
ALTER TABLE ssh_keys ADD COLUMN fingerprint TEXT NOT NULL DEFAULT '';
