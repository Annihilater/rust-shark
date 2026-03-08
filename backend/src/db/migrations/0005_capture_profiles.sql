-- 抓包配置表（配置模板，可复用）
CREATE TABLE IF NOT EXISTS capture_profiles (
    id          TEXT PRIMARY KEY NOT NULL,
    user_id     TEXT NOT NULL,
    name        TEXT NOT NULL,
    server_id   TEXT NOT NULL,
    interface   TEXT NOT NULL,
    filter      TEXT,
    duration    INTEGER,
    packet_limit INTEGER,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id)   REFERENCES users(id)   ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_capture_profiles_user_id ON capture_profiles(user_id);

-- 抓包任务新增 profile_id 字段（可为 NULL，支持不使用配置直接创建）
ALTER TABLE capture_tasks ADD COLUMN profile_id TEXT REFERENCES capture_profiles(id) ON DELETE SET NULL;
