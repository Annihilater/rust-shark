# ─── 直接打包预编译二进制，不在镜像内重新编译 ─────────────────────────────────
#
# CI 流程：
#   build-backend job 用 cross 交叉编译出静态链接 musl 二进制
#   build-docker job 下载产物后调用此 Dockerfile
#
# 构建时必须通过 --build-arg TARGETARCH=amd64|arm64 指定目标架构
# docker/build-push-action 在多平台构建时会自动注入 TARGETARCH
#
FROM debian:bookworm-slim

ARG TARGETARCH

# DEBIAN_FRONTEND=noninteractive：避免 wireshark-common 安装时弹出交互式提示
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates wget \
    wireshark-common \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 直接复制预编译好的静态链接二进制（CI 已完成交叉编译，此处仅打包）
# 构建上下文中已有：
#   rust-shark-linux-amd64  （x86_64-unknown-linux-musl）
#   rust-shark-linux-arm64  （aarch64-unknown-linux-musl）
COPY rust-shark-linux-${TARGETARCH} /usr/local/bin/rust-shark
RUN chmod +x /usr/local/bin/rust-shark

# 创建数据目录
RUN mkdir -p /app/data/captures

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost:${PORT:-17823}/api/health || exit 1

EXPOSE 17823

ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=17823
ENV DATABASE_URL=sqlite:///app/data/rust-shark.db
ENV DATA_DIR=/app/data

CMD ["/usr/local/bin/rust-shark"]
