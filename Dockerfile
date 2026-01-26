# ----------- Stage 1: Build Rust binary -----------
FROM rust:1.91 as builder

WORKDIR /app

# 复制 Cargo.toml 和 Cargo.lock，先构建依赖
COPY Cargo.toml Cargo.lock ./
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs
RUN cargo build --release
# 这一步只是为了缓存依赖

# 复制源码
COPY . .
RUN cargo build --release

# ----------- Stage 2: Runtime -----------
FROM ubuntu:24.04

# 安装 LibreOffice + 字体
RUN apt-get update && apt-get install -y \
    libreoffice \
    libreoffice-writer \
    libreoffice-calc \
    libreoffice-impress \
    fonts-noto \
    fonts-noto-cjk \
    fonts-dejavu \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 复制 Rust 可执行文件
COPY --from=builder /app/target/release/yo-office-preview /app/yo_office_preview
RUN chmod +x /app/yo_office_preview

ENV LIBREOFFICE_BIN=/usr/bin/libreoffice
ENV TMP_DIR=/tmp/office-preview
ENV PORT=3000

# 暴露端口
EXPOSE 3000

# 默认命令
CMD ["./yo_office_preview"]
