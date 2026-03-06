-- 用户表
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    email TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- SSH 密钥表
CREATE TABLE IF NOT EXISTS ssh_keys (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    private_key_encrypted TEXT NOT NULL,
    public_key TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 服务器表
CREATE TABLE IF NOT EXISTS servers (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 22,
    username TEXT NOT NULL,
    auth_type TEXT NOT NULL DEFAULT 'key',
    ssh_key_id TEXT,
    password_encrypted TEXT,
    status TEXT NOT NULL DEFAULT 'unknown',
    last_checked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (ssh_key_id) REFERENCES ssh_keys(id) ON DELETE SET NULL
);

-- 抓包任务表
CREATE TABLE IF NOT EXISTS capture_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    server_id TEXT NOT NULL,
    interface TEXT NOT NULL,
    filter TEXT,
    duration INTEGER,
    packet_limit INTEGER,
    status TEXT NOT NULL DEFAULT 'pending',
    file_path TEXT,
    file_size INTEGER,
    error_msg TEXT,
    scheduled_at TEXT,
    repeat_type TEXT NOT NULL DEFAULT 'none',
    repeat_until TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE CASCADE
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_ssh_keys_user_id ON ssh_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_servers_user_id ON servers(user_id);
CREATE INDEX IF NOT EXISTS idx_capture_tasks_user_id ON capture_tasks(user_id);
CREATE INDEX IF NOT EXISTS idx_capture_tasks_server_id ON capture_tasks(server_id);
CREATE INDEX IF NOT EXISTS idx_capture_tasks_status ON capture_tasks(status);
