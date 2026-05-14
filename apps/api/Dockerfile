# ── Token Leaderboard API ─────────────────────────────────
# 构建: docker build -f apps/api/Dockerfile .

FROM rust:latest AS builder
WORKDIR /app

# cargo 国内镜像源（中科大）
RUN mkdir -p /usr/local/cargo && \
    printf '[source.crates-io]\nreplace-with = "ustc"\n\n[source.ustc]\nregistry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"\n' > /usr/local/cargo/config.toml && \
    echo "── cargo 源: $(head -1 /usr/local/cargo/config.toml) → USTC 镜像"

COPY Cargo.toml Cargo.lock ./
COPY db db
COPY crates/common crates/common
COPY apps/cli apps/cli
COPY apps/api apps/api
RUN CARGO_TERM_PROGRESS_WHEN=always CARGO_TERM_PROGRESS_WIDTH=80 cargo build -p api --release -j 16

FROM debian:bookworm-slim
RUN sed -i 's|deb.debian.org|mirrors.ustc.edu.cn|g; s|security.debian.org|mirrors.ustc.edu.cn|g' /etc/apt/sources.list.d/debian.sources 2>/dev/null || \
    sed -i 's|deb.debian.org|mirrors.ustc.edu.cn|g' /etc/apt/sources.list && \
    echo "── apt 源: 替换为 USTC 镜像" && \
    apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/api /usr/local/bin/api
EXPOSE 8080
CMD ["api"]
