# rust-shark 设计文档

> 日期：2026-03-06
> 项目：全栈 Rust 抓包管理平台

## 概述

rust-shark 是一个多用户 SaaS 抓包管理平台，用户可以通过 Web 界面管理服务器、SSH 密钥，发起远程抓包任务，下载 pcap 文件，并直接在浏览器中通过 sharkd 分析数据包内容。

## 技术栈

| 层 | 技术 |
|---|---|
| 后端框架 | Axum |
| 数据库 | SQLite (sqlx) |
| SSH 连接 | russh (pure Rust) |
| 前端框架 | Leptos (Rust → WASM) |
| 前端样式 | TailwindCSS |
| 静态资源嵌入 | rust-embed |
| 认证 | JWT + bcrypt |
| 数据包分析 | sharkd (Wireshark daemon) |
| 构建工具 | trunk (前端) + cargo (后端) |
| 任务调度 | tokio-cron-scheduler |

## 单二进制架构

```
rust-shark (单一可执行文件)
  ├── 嵌入 Leptos WASM 前端 (dist/)
  ├── 嵌入 tcpdump 各平台二进制
  │   ├── tcpdump-linux-amd64
  │   └── tcpdump-linux-arm64
  ├── 嵌入 sharkd 各平台二进制
  │   ├── sharkd-linux-amd64
  │   └── sharkd-linux-arm64
  └── Axum HTTP server
```

## 数据模型

```sql
-- 用户表
users
  id          TEXT PRIMARY KEY
  email       TEXT UNIQUE NOT NULL
  name        TEXT NOT NULL
  password_hash TEXT NOT NULL
  role        TEXT NOT NULL  -- admin | user
  created_at  DATETIME NOT NULL

-- SSH 密钥表
ssh_keys
  id          TEXT PRIMARY KEY
  user_id     TEXT NOT NULL  -- FK users.id
  name        TEXT NOT NULL
  private_key TEXT NOT NULL  -- AES-256-GCM 加密存储
  public_key  TEXT NOT NULL
  created_at  DATETIME NOT NULL

-- 服务器表
servers
  id          TEXT PRIMARY KEY
  user_id     TEXT NOT NULL  -- FK users.id
  name        TEXT NOT NULL
  host        TEXT NOT NULL
  port        INTEGER NOT NULL DEFAULT 22
  username    TEXT NOT NULL
  auth_type   TEXT NOT NULL  -- key | password
  ssh_key_id  TEXT           -- nullable, FK ssh_keys.id
  password_encrypted TEXT    -- nullable, AES-256-GCM 加密
  status      TEXT NOT NULL DEFAULT 'unknown'  -- unknown | online | offline
  created_at  DATETIME NOT NULL

-- 抓包任务表
capture_tasks
  id          TEXT PRIMARY KEY
  user_id     TEXT NOT NULL  -- FK users.id
  server_id   TEXT NOT NULL  -- FK servers.id
  interface   TEXT NOT NULL  -- 网卡名称 eth0/ens3 等
  filter      TEXT           -- BPF 过滤语法
  duration    INTEGER        -- 秒，nullable=不限时
  packet_limit INTEGER       -- 包数限制，nullable=不限
  status      TEXT NOT NULL DEFAULT 'pending'
              -- pending | running | done | failed | cancelled
  file_path   TEXT           -- 后端存储路径
  file_size   INTEGER        -- bytes
  error_msg   TEXT           -- 失败原因
  scheduled_at DATETIME      -- nullable=立即执行
  repeat_type TEXT DEFAULT 'none'  -- none | daily | weekly
  repeat_until DATETIME      -- 重复截止时间
  created_at  DATETIME NOT NULL
  finished_at DATETIME
```

## 前端页面结构

```
/ (登录页)
/dashboard              — 概览，抓包任务状态统计
/servers                — 服务器列表
/servers/new            — 添加服务器
/servers/:id            — 服务器详情 + 发起抓包
/keys                   — SSH 密钥管理
/keys/new               — 添加密钥（粘贴或生成）
/captures               — 抓包任务列表 + 下载
/captures/:id/analyze   — 数据包分析（sharkd 集成）
/admin/users            — 用户管理（仅 admin）
```

## 后端 API

```
POST   /api/auth/login
POST   /api/auth/logout

GET    /api/keys                    — 密钥列表
POST   /api/keys                    — 添加密钥
DELETE /api/keys/:id                — 删除密钥

GET    /api/servers                 — 服务器列表
POST   /api/servers                 — 添加服务器
PUT    /api/servers/:id             — 更新服务器
DELETE /api/servers/:id             — 删除服务器
POST   /api/servers/:id/test        — 测试连接 + 检测/安装 tcpdump
GET    /api/servers/:id/interfaces  — 获取网卡列表

GET    /api/captures                — 抓包任务列表
POST   /api/captures                — 创建任务（立即或定时）
GET    /api/captures/:id            — 任务详情
DELETE /api/captures/:id            — 删除任务
POST   /api/captures/:id/stop       — 停止抓包
GET    /api/captures/:id/download   — 下载 pcap 文件
POST   /api/captures/:id/analyze    — 启动 sharkd 分析
GET    /api/captures/:id/packets    — 获取数据包列表（sharkd）
GET    /api/captures/:id/packet/:no — 获取单包详情（sharkd）

GET    /api/admin/users             — 用户列表
POST   /api/admin/users             — 创建用户
DELETE /api/admin/users/:id         — 删除用户
```

## tcpdump 自动部署流程

```
1. SSH 连接目标服务器
2. which tcpdump → 检测是否已安装
3. 未安装 → 检测包管理器（apt/yum/apk）
4. 尝试包管理器安装（sudo apt install tcpdump -y）
5. 安装失败 → 上传内嵌对应平台二进制到 /tmp/tcpdump-rs
6. chmod +x /tmp/tcpdump-rs
7. 验证可执行
8. 开始抓包
```

## 抓包执行流程

```
1. 创建任务记录（status=pending）
2. 定时任务或立即 → 触发执行
3. SSH 连接 → sudo tcpdump -i {interface} -w /tmp/{task_id}.pcap {filter} [-c limit] [-G duration]
4. 异步监控进程状态（status=running）
5. 达到限制或手动停止 → 终止进程
6. SCP 拉取 /tmp/{task_id}.pcap → data/captures/{user_id}/{task_id}.pcap
7. 清理服务器临时文件
8. 更新 status=done，记录 file_size, finished_at
```

## sharkd 数据包分析流程

```
1. 用户点击"分析"
2. 后端启动 sharkd 进程（内嵌二进制）
3. sharkd 通过 Unix socket 通信
4. 前端调用 /api/captures/:id/packets 获取包列表
5. 前端支持 Wireshark display filter 语法过滤
6. 点击单包 → /api/captures/:id/packet/:no 获取协议树 + 原始数据
7. 分析完成 → 关闭 sharkd 进程
```

## 安全设计

- **SSH 密钥/密码加密**：AES-256-GCM，密钥从环境变量 `SECRET_KEY` 读取
- **JWT 认证**：24h 过期，每个 API 校验 user_id
- **数据隔离**：所有查询强制 WHERE user_id = ?
- **路径安全**：文件路径防穿越（path traversal 保护）
- **SSH 连接**：优先密钥认证，连接超时 30s

## 跨平台编译

### 本机（M1 macOS）开发
```bash
# 前端
trunk build --release

# 后端（本地测试用）
cargo build --release

# 交叉编译 Linux amd64（生产目标）
cargo build --release --target x86_64-unknown-linux-musl

# 交叉编译 Linux arm64
cargo build --release --target aarch64-unknown-linux-musl
```

### 内嵌二进制平台矩阵
| 目标平台 | tcpdump | sharkd |
|---|---|---|
| linux/amd64 | ✓ | ✓ |
| linux/arm64 | ✓ | ✓ |

## GitHub Actions 流水线

### 触发条件
- Push tag `v*.*.*` → 发布 Release
- PR/Push main → 构建验证

### 构建矩阵
```yaml
targets:
  - x86_64-unknown-linux-musl    # Linux amd64
  - aarch64-unknown-linux-musl   # Linux arm64
```

### Release 制品
```
rust-shark-linux-amd64          (静态链接二进制)
rust-shark-linux-arm64          (静态链接二进制)
rust-shark-linux-amd64.tar.gz
rust-shark-linux-arm64.tar.gz
```

## 目录结构

```
rust-shark/
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── db/
│   │   ├── mod.rs
│   │   └── migrations/
│   ├── api/
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── servers.rs
│   │   ├── keys.rs
│   │   ├── captures.rs
│   │   └── admin.rs
│   ├── services/
│   │   ├── ssh.rs
│   │   ├── capture.rs
│   │   ├── scheduler.rs
│   │   ├── sharkd.rs
│   │   └── crypto.rs
│   └── models/
│       ├── user.rs
│       ├── server.rs
│       ├── ssh_key.rs
│       └── capture.rs
├── frontend/
│   ├── src/
│   │   ├── main.rs
│   │   ├── pages/
│   │   └── components/
│   ├── Trunk.toml
│   └── index.html
├── assets/
│   └── binaries/
│       ├── tcpdump-linux-amd64
│       ├── tcpdump-linux-arm64
│       ├── sharkd-linux-amd64
│       └── sharkd-linux-arm64
├── deploy/
│   ├── docker-compose.yml
│   ├── data/
│   ├── common.sh
│   ├── start.sh
│   ├── stop.sh
│   ├── restart.sh
│   ├── exec.sh
│   ├── logs.sh
│   ├── status.sh
│   └── pull.sh
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
├── justfile
├── Dockerfile
├── Cargo.toml
└── Cargo.lock
```

## 实现阶段

### 阶段 1: 项目骨架
**目标**: 可编译的前后端骨架，路由通，数据库初始化
**状态**: 未开始

### 阶段 2: 认证 + 用户管理
**目标**: 登录/登出，JWT 中间件，admin 用户管理
**状态**: 未开始

### 阶段 3: SSH 密钥 + 服务器管理
**目标**: 密钥 CRUD，服务器 CRUD，SSH 连接测试，网卡获取
**状态**: 未开始

### 阶段 4: 抓包核心
**目标**: tcpdump 检测/安装，抓包执行，文件下载，定时任务
**状态**: 未开始

### 阶段 5: sharkd 数据包分析
**目标**: sharkd 集成，前端数据包展示，过滤器
**状态**: 未开始

### 阶段 6: 构建 + CI/CD
**目标**: 多平台编译，Docker 单二进制，GitHub Actions Release
**状态**: 未开始
