# Yo-Office-Preview ⚡️

基于 Rust + LibreOffice 的文档预览服务。使用 `axum` + `tokio` 提供 HTTP 接口，支持将 Office 文档转换为 PDF 并提供预览或下载。

---

## 🔧 技术栈与依赖

- 语言：Rust（async runtime: `tokio`）
- Web 框架：`axum`
- 缓存：`moka`
- 文档转换：调用系统安装的 LibreOffice（headless 模式）
- 目录结构（关键文件）：
  - `src/main.rs`：程序入口、路由与配置
  - `src/preview.rs`：预览接口实现
  - `src/convert.rs`：下载与调用 LibreOffice 转换逻辑
  - `src/cache.rs`：缓存实现
  - `src/url_helper.rs`：URL 判断与工具函数
  - `static/index.html`：静态说明页
  - `Dockerfile`：镜像构建配置

---

## ✅ 功能概览

- 支持将远程或本地 Office 文档（`.doc/.docx/.xls/.xlsx/.ppt/.pptx` 等）转换为 PDF。
- 提供 `/preview?url=...` 接口，返回 PDF（二进制，`Content-Type: application/pdf`）。
- 转换结果本地缓存（默认 1 小时）以提升性能。
- 支持通过 HTTP 下载或在浏览器中直接预览。

---

## ⚙️ 环境变量（常用）

- `LIBREOFFICE_PATH`（必需）: LibreOffice 可执行文件路径（如 `/usr/bin/libreoffice` 或容器内的可执行路径）。
- `OUTPUT_DIR`（可选）: 转换输出目录，默认使用系统临时目录。
- `PORT`（可选）: 服务监听端口，默认 `3000`。

> 提示：如果仓库中有 `.env.example`，可复制为 `.env` 并按需修改。

---

## ▶️ 本地运行（开发）

1. 安装 Rust（和 Cargo）以及系统级依赖 LibreOffice。
2. 配置环境变量：
```sh
cp .env.example .env  # 如果存在
# 编辑 .env 或在环境中设置 LIBREOFFICE_PATH 等
```
3. 直接运行（开发或调试）：
```sh
cargo run
```
4. 生产模式运行（性能更佳）：
```sh
cargo build --release
./target/release/yo-office-preview
```

---

## 🧪 测试

运行单元测试：
```sh
cargo test
```

---

## 📦 构建 Docker 镜像

构建镜像（示例 tag）：
```sh
docker build -t yo-office-preview:latest .
```

运行容器：
```sh
docker run -p 3000:3000 --env-file .env yo-office-preview:latest
```

将输出目录持久化：
```sh
docker run -p 3000:3000 -v /host/output:/tmp --env-file .env yo-office-preview:latest
```

---

## 🧭 使用示例

通过 `curl` 请求并保存返回的 PDF：
```sh
curl "http://localhost:3000/preview?url=https://example.com/test.docx" --output preview.pdf
```

浏览器访问：
- 打开 `http://localhost:3000/` 查看静态说明页（若启用）。

---

## ⚠️ 注意事项

- 必须在运行环境中安装可用的 LibreOffice，并且 `LIBREOFFICE_PATH` 指向正确的可执行文件。
- 远程文件会先下载并根据 URL / HTTP 头判断扩展名；不支持或未知格式会导致转换失败。
- 缓存策略和超时可在 `src/cache.rs` 中查看与调整。

---

## 📚 参考

- 代码入口与实现请参阅 `src/` 目录下的相关文件（`main.rs`, `convert.rs`, `preview.rs`, `cache.rs`, `url_helper.rs`）。

---

感谢使用！如需增加 API、支持更多格式或改进缓存策略，欢迎发起 Issue 或 PR。 ✅
