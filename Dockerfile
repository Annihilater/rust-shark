# ─── Stage 1: 构建后端 ─────────────────────────────────────────────────────────
# 前端 WASM 由 CI 的 build-frontend job 预先构建并注入到 Docker build context 中
# (frontend/dist/ 已存在于上下文，无需在此重新构建)
FROM rust:1.85-slim AS backend-builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制全部源码（含 CI 注入的 frontend/dist/ 和 assets/binaries/）
COPY . .

# 构建后端（嵌入前端静态资源）
RUN cargo build --release --manifest-path backend/Cargo.toml

# ─── Stage 2: 最终运行镜像 ─────────────────────────────────────────────────────
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
