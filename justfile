# rust-shark justfile

set dotenv-load := true

# 默认任务
default:
    @just --list

# ─── 开发 ─────────────────────────────────────────────────────────────────────

# 启动后端开发服务器
dev-backend:
    cd backend && cargo run

# 启动前端开发服务器
dev-frontend:
    cd frontend && trunk serve

# 同时启动前后端（需要 tmux 或两个终端）
dev:
    @echo "请在两个终端分别运行: just dev-backend 和 just dev-frontend"

# ─── 构建 ─────────────────────────────────────────────────────────────────────

# 构建前端（WASM）
build-frontend:
    cd frontend && trunk build --release

# 构建后端（本机）
build-backend: build-frontend
    cd backend && cargo build --release

# 构建后端 Linux amd64（使用 cross）
build-linux-amd64: build-frontend
    cross build --release --target x86_64-unknown-linux-musl --manifest-path backend/Cargo.toml

# 构建后端 Linux arm64（使用 cross）
build-linux-arm64: build-frontend
    cross build --release --target aarch64-unknown-linux-musl --manifest-path backend/Cargo.toml

# 构建全部平台
build-all: build-linux-amd64 build-linux-arm64

# ─── 数据库 ───────────────────────────────────────────────────────────────────

# 创建 admin 初始用户
create-admin email password="Admin@123456":
    @echo "创建管理员用户: {{email}}"
    cd backend && cargo run --bin create-admin -- {{email}} {{password}}

# ─── Docker ───────────────────────────────────────────────────────────────────

# 构建 Docker 镜像
docker-build:
    docker build -t rust-shark:latest .

# 启动服务
start:
    ./deploy/start.sh

# 停止服务
stop:
    ./deploy/stop.sh

# 重启服务
restart:
    ./deploy/restart.sh

# 查看日志
logs *args:
    ./deploy/logs.sh {{args}}

# 查看状态
status:
    ./deploy/status.sh

# 进入容器
exec:
    ./deploy/exec.sh

# 拉取镜像
pull:
    ./deploy/pull.sh

# ─── 代码质量 ─────────────────────────────────────────────────────────────────

# 格式化代码
fmt:
    cargo fmt --all

# 代码检查
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# 运行测试
test:
    cargo test --all

# 完整检查
check: fmt lint test
    @echo "✓ 所有检查通过"
