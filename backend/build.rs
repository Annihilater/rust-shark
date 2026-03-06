// build.rs: 复制 assets 到 OUT_DIR，处理空文件情况
use std::path::Path;

fn main() {
    // 告诉 cargo 监听 assets 目录变化
    println!("cargo:rerun-if-changed=assets/binaries/");
    println!("cargo:rerun-if-changed=../frontend/dist/");

    // 确保前端 dist 目录存在（开发时可能未构建）
    let dist = Path::new("../frontend/dist");
    if !dist.exists() {
        std::fs::create_dir_all(dist).ok();
        // 创建占位 index.html
        std::fs::write(dist.join("index.html"), b"<html><body>Building...</body></html>").ok();
    }

    // 确保 assets/binaries 存在
    let assets = Path::new("../assets/binaries");
    if !assets.exists() {
        std::fs::create_dir_all(assets).ok();
    }

    // 为空的二进制文件写入一个最小占位（1字节），避免 include_bytes! 因空文件报错
    for name in &[
        "tcpdump-linux-amd64",
        "tcpdump-linux-arm64",
        "sharkd-linux-amd64",
        "sharkd-linux-arm64",
    ] {
        let path = assets.join(name);
        if path.exists() {
            let meta = std::fs::metadata(&path).unwrap();
            if meta.len() == 0 {
                std::fs::write(&path, b"\x00").ok();
            }
        } else {
            std::fs::write(&path, b"\x00").ok();
        }
    }
}
