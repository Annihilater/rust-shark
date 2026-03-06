# ─── Stage 1: 构建前端 WASM ────────────────────────────────────────────────────
FROM rust:1.85-slim AS frontend-builder

WORKDIR /app

# 安装 trunk 和 wasm 目标
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk --locked

# 安装 Node/wasm-bindgen 依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev curl \
    && rm -rf /var/lib/apt/lists/*

# 复制前端代码
COPY frontend/ ./frontend/
COPY Cargo.toml ./
COPY frontend/Cargo.toml ./frontend/Cargo.toml

# 构建前端
WORKDIR /app/frontend
RUN trunk build --release

# ─── Stage 2: 构建后端 ─────────────────────────────────────────────────────────
FROM rust:1.85-slim AS backend-builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev musl-tools \
    && rm -rf /var/lib/apt/lists/*

# 复制全部源码
COPY . .
# 复制前端构建产物
COPY --from=frontend-builder /app/frontend/dist/ ./frontend/dist/

# 构建后端（嵌入前端）
RUN cargo build --release --manifest-path backend/Cargo.toml

# ─── Stage 3: 最终运行镜像 ─────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 复制二进制
COPY --from=backend-builder /app/target/release/rust-shark /usr/local/bin/rust-shark

# 创建数据目录
RUN mkdir -p /app/data/captures

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost:3000/api/health || exit 1

EXPOSE 3000

ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=3000
ENV DATABASE_URL=sqlite:///app/data/rust-shark.db
ENV DATA_DIR=/app/data

CMD ["/usr/local/bin/rust-shark"]
