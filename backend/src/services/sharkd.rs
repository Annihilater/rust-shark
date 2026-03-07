use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::info;

// 内嵌 sharkd 二进制（Linux 用）
#[cfg(target_arch = "x86_64")]
const SHARKD_BINARY: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/binaries/sharkd-linux-amd64"));

#[cfg(target_arch = "aarch64")]
const SHARKD_BINARY: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/binaries/sharkd-linux-arm64"));

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const SHARKD_BINARY: &[u8] = &[0u8];

#[derive(Debug, Serialize, Deserialize)]
pub struct PacketSummary {
    pub number: u64,
    pub time: String,
    pub source: String,
    pub destination: String,
    pub protocol: String,
    pub length: u64,
    pub info: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PacketDetail {
    pub number: u64,
    pub layers: serde_json::Value,
    pub raw: String,
}

pub struct SharkdSession {
    sharkd_path: String,
    pcap_path: String,
}

impl SharkdSession {
    pub async fn new(pcap_path: &str, data_dir: &str) -> Result<Self> {
        let sharkd_path = ensure_sharkd(data_dir).await?;
        // 确保 pcap 路径是绝对路径，sharkd 需要绝对路径才能打开文件
        let abs_pcap = if std::path::Path::new(pcap_path).is_absolute() {
            PathBuf::from(pcap_path)
        } else {
            // 相对路径：基于当前工作目录拼接
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            cwd.join(pcap_path)
        };
        Ok(Self {
            sharkd_path,
            pcap_path: abs_pcap.to_string_lossy().to_string(),
        })
    }

    /// 获取数据包列表
    pub async fn get_packets(
        &self,
        skip: u64,
        limit: u64,
        filter: Option<&str>,
    ) -> Result<Vec<PacketSummary>> {
        // 构造 JSON-RPC 2.0 请求
        let mut frames_params = serde_json::json!({ "limit": limit });
        if skip > 0 {
            frames_params["skip"] = serde_json::Value::Number(skip.into());
        }
        if let Some(f) = filter {
            if !f.is_empty() {
                frames_params["filter"] = serde_json::Value::String(f.to_string());
            }
        }

        let load_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "load",
            "params": { "file": self.pcap_path }
        });
        let frames_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "frames",
            "params": frames_params
        });

        let input = format!("{}\n{}\n", load_req, frames_req);
        let stdout = self.run_sharkd(&input).await?;

        // 解析 JSON-RPC 响应：找 id=2 的那行
        let result = extract_result(&stdout, 2)?;

        // frames 结果是数组，每项有 "c"(columns) 和 "num"
        // c: [no_str, time, src, dst, protocol, length_str, info]
        let packets = result
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let c = item.get("c")?.as_array()?;
                        let num = item.get("num")?.as_u64().unwrap_or(0);
                        Some(PacketSummary {
                            number: num,
                            time: c.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            source: c.get(2).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            destination: c.get(3).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            protocol: c.get(4).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            length: c.get(5).and_then(|v| v.as_str()).unwrap_or("0").parse().unwrap_or(0),
                            info: c.get(6).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(packets)
    }

    /// 获取单包详情
    pub async fn get_packet_detail(&self, frame_number: u64) -> Result<PacketDetail> {
        let load_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "load",
            "params": { "file": self.pcap_path }
        });
        let frame_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "frame",
            "params": {
                "frame": frame_number,
                "proto": true,
                "bytes": true
            }
        });

        let input = format!("{}\n{}\n", load_req, frame_req);
        let stdout = self.run_sharkd(&input).await?;

        let result = extract_result(&stdout, 2)?;

        Ok(PacketDetail {
            number: frame_number,
            layers: result.get("tree").cloned().unwrap_or(serde_json::Value::Array(vec![])),
            raw: result
                .get("bytes")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    async fn run_sharkd(&self, input: &str) -> Result<String> {
        let mut child = Command::new(&self.sharkd_path)
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("启动 sharkd 失败")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).await?;
            stdin.flush().await?;
            drop(stdin);
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            child.wait_with_output(),
        )
        .await
        .context("sharkd 执行超时")??;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// 从 sharkd 多行输出中找指定 id 的响应，提取 result 字段
fn extract_result(stdout: &str, id: u64) -> Result<serde_json::Value> {
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            // 检查 id 匹配
            if val.get("id").and_then(|v| v.as_u64()) == Some(id) {
                if let Some(result) = val.get("result") {
                    return Ok(result.clone());
                }
                // 有 error 字段
                if let Some(err) = val.get("error") {
                    anyhow::bail!("sharkd error: {}", err);
                }
            }
        }
    }
    // 找不到则返回空数组（对 frames 安全）
    Ok(serde_json::Value::Array(vec![]))
}

async fn ensure_sharkd(data_dir: &str) -> Result<String> {
    let sharkd_path = format!("{}/sharkd", data_dir);

    // 先检查系统 sharkd（开发机 Mac 上优先用系统的）
    if let Ok(output) = Command::new("which").arg("sharkd").output().await {
        let sys_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !sys_path.is_empty() && Path::new(&sys_path).exists() {
            info!("使用系统 sharkd: {}", sys_path);
            return Ok(sys_path);
        }
    }

    // 检查已解压的内嵌二进制
    if Path::new(&sharkd_path).exists() {
        let meta = tokio::fs::metadata(&sharkd_path).await?;
        if meta.len() > 1024 {
            return Ok(sharkd_path);
        }
        // 空文件或太小，删除重建
        tokio::fs::remove_file(&sharkd_path).await.ok();
    }

    // 写出内嵌二进制
    if SHARKD_BINARY.len() > 1024 {
        tokio::fs::create_dir_all(data_dir).await?;
        tokio::fs::write(&sharkd_path, SHARKD_BINARY).await?;
        Command::new("chmod").args(["+x", &sharkd_path]).output().await?;
        info!("sharkd 二进制已释放到: {}", sharkd_path);
        return Ok(sharkd_path);
    }

    anyhow::bail!("sharkd 不可用，请安装 Wireshark (brew install wireshark)")
}
