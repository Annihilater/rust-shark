use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;
use tracing::info;

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
        // 确保 sharkd 二进制存在
        let sharkd_path = ensure_sharkd(data_dir).await?;
        Ok(Self {
            sharkd_path,
            pcap_path: pcap_path.to_string(),
        })
    }

    /// 获取数据包列表
    pub async fn get_packets(
        &self,
        skip: u64,
        limit: u64,
        filter: Option<&str>,
    ) -> Result<Vec<PacketSummary>> {
        let filter_arg = filter.unwrap_or("");
        let cmd_json = serde_json::json!({
            "req": "frames",
            "skip": skip,
            "limit": limit,
            "filter": filter_arg
        });

        let output = self.run_sharkd_cmd(&cmd_json).await?;

        // 解析 sharkd 返回格式
        let packets: Vec<PacketSummary> = serde_json::from_value(output)
            .unwrap_or_default();

        Ok(packets)
    }

    /// 获取单包详情
    pub async fn get_packet_detail(&self, frame_number: u64) -> Result<PacketDetail> {
        let cmd_json = serde_json::json!({
            "req": "frame",
            "frame": frame_number,
            "proto": true,
            "bytes": true
        });

        let output = self.run_sharkd_cmd(&cmd_json).await?;

        Ok(PacketDetail {
            number: frame_number,
            layers: output.get("tree").cloned().unwrap_or(serde_json::Value::Null),
            raw: output
                .get("bytes")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    async fn run_sharkd_cmd(&self, cmd: &serde_json::Value) -> Result<serde_json::Value> {
        // sharkd 单次模式：通过 stdin/stdout 通信
        let _input = format!(
            "{{\"req\":\"load\",\"file\":\"{}\"}}\n{}\n",
            self.pcap_path,
            cmd.to_string()
        );

        let output = Command::new(&self.sharkd_path)
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("启动 sharkd 失败")?
            .wait_with_output()
            .await?;

        // 解析最后一行 JSON 输出
        let stdout = String::from_utf8_lossy(&output.stdout);
        let last_line = stdout.lines().last().unwrap_or("{}");
        let value: serde_json::Value = serde_json::from_str(last_line)?;

        Ok(value)
    }
}

async fn ensure_sharkd(data_dir: &str) -> Result<String> {
    let sharkd_path = format!("{}/sharkd", data_dir);

    // 如果已存在且可执行，直接返回
    if Path::new(&sharkd_path).exists() {
        return Ok(sharkd_path);
    }

    if SHARKD_BINARY.is_empty() {
        // 尝试系统已安装的 sharkd
        if let Ok(output) = Command::new("which").arg("sharkd").output().await {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
        anyhow::bail!("sharkd 不可用，请安装 Wireshark");
    }

    // 写出内嵌二进制
    tokio::fs::create_dir_all(data_dir).await?;
    tokio::fs::write(&sharkd_path, SHARKD_BINARY).await?;

    Command::new("chmod")
        .args(["+x", &sharkd_path])
        .output()
        .await?;

    info!("sharkd 二进制已释放到: {}", sharkd_path);
    Ok(sharkd_path)
}
