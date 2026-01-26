# ----------- Stage 1: Build Rust binary -----------
FROM rust:1.91 AS builder

WORKDIR /app
COPY ./src ./src
COPY ./static ./static
COPY Cargo.toml ./Cargo.toml
COPY Cargo.lock ./Cargo.lock
RUN cargo build --release

# ----------- Stage 2: Runtime -----------
FROM ubuntu:24.04

WORKDIR /app

# 复制 Rust 可执行文件
COPY --from=builder /app/target/release/yo-office-preview /app/yo_office_preview
COPY --from=builder /app/static /app/static
RUN chmod +x /app/yo_office_preview

ENV DEBIAN_FRONTEND=noninteractive

# 安装 LibreOffice + 字体
# libreoffice-writer \      # 支持 DOC/DOCX
# libreoffice-calc \        # 支持 XLS/XLSX
# libreoffice-impress \     # 支持 PPT/PPTX
# fonts-noto-cjk \          # 中日韩字体（可选，但推荐）
# Writer: .doc, .docx, .odt, .rtf, .txt → PDF
# Calc: .xls, .xlsx, .ods → PDF
# Impress: .ppt, .pptx, .odp → PDF
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        libreoffice \
        libreoffice-writer \
        libreoffice-calc \
        libreoffice-impress \
        fonts-liberation \
        fonts-dejavu-core \
        fonts-noto-cjk \
        && rm -rf /var/lib/apt/lists/*

ENV LIBREOFFICE_PATH=libreoffice
ENV TMP_DIR=tmp
ENV PORT=3000

# 暴露端口
EXPOSE 3000

# 默认命令
CMD ["./yo_office_preview"]
