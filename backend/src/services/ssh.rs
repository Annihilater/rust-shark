use crate::models::server::{NetworkInterface, ServerTestResult};
use anyhow::{Context, Result};
use openssh::{KnownHosts, Session, SessionBuilder};
use std::io::Write;
use tempfile::NamedTempFile;

/// 构建 SSH Session，返回 (Session, 临时密钥文件)
pub async fn connect(
    host: &str,
    port: u16,
    username: &str,
    auth_type: &str,
    private_key_pem: Option<&str>,
    _password: Option<&str>,
) -> Result<(Session, Option<NamedTempFile>)> {
    let mut builder = SessionBuilder::default();
    builder
        .user(username.to_string())
        .port(port)
        .known_hosts_check(KnownHosts::Accept)
        .connect_timeout(std::time::Duration::from_secs(30))
        .server_alive_interval(std::time::Duration::from_secs(10));

    let mut key_file: Option<NamedTempFile> = None;

    if auth_type == "key" {
        let pem = private_key_pem.context("密钥认证需要私钥")?;
        let mut tmp = NamedTempFile::new()?;
        tmp.write_all(pem.as_bytes())?;
        tmp.flush()?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600))?;
        }

        builder.keyfile(tmp.path());
        key_file = Some(tmp);
    }

    let dest = format!("{}@{}", username, host);
    let session = builder
        .connect(&dest)
        .await
        .context(format!("SSH连接 {}:{} 失败", host, port))?;

    Ok((session, key_file))
}

/// 在远端执行命令，返回 (stdout, exit_code)
pub async fn exec(session: &Session, command: &str) -> Result<(String, i32)> {
    let output = session
        .command("sh")
        .arg("-c")
        .arg(command)
        .output()
        .await
        .context(format!(
            "执行命令失败: {}",
            &command[..command.len().min(100)]
        ))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok((stdout, exit_code))
}

/// 测试服务器连接并检查 tcpdump
pub async fn test_server_connection(
    host: &str,
    port: u16,
    username: &str,
    auth_type: &str,
    private_key_pem: Option<&str>,
    password: Option<&str>,
) -> Result<ServerTestResult> {
    let (session, _key_file) =
        connect(host, port, username, auth_type, private_key_pem, password).await?;

    // 检测 tcpdump
    let (stdout, code) = exec(&session, "which tcpdump 2>/dev/null || echo ''").await?;
    let tcpdump_path = stdout.trim().to_string();
    let tcpdump_available = code == 0 && !tcpdump_path.is_empty();

    let tcpdump_version = if tcpdump_available {
        let (ver, _) = exec(&session, "tcpdump --version 2>&1 | head -1").await?;
        Some(ver.trim().to_string())
    } else {
        None
    };

    // 获取网卡列表
    let (iface_out, _) = exec(
        &session,
        "ip link show 2>/dev/null | grep -E '^[0-9]+:' | awk -F': ' '{print $2}' | awk '{print $1}' | tr -d '@'",
    )
    .await?;

    let interfaces: Vec<NetworkInterface> = iface_out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|name| NetworkInterface {
            name: name.trim().to_string(),
            description: None,
        })
        .collect();

    session.close().await.ok();

    Ok(ServerTestResult {
        success: true,
        message: "连接成功".to_string(),
        tcpdump_available,
        tcpdump_version,
        interfaces,
    })
}

/// 获取服务器上正在监听的 TCP/UDP 端口列表（排序去重）
pub async fn get_listening_ports(
    host: &str,
    port: u16,
    username: &str,
    auth_type: &str,
    private_key_pem: Option<&str>,
    password: Option<&str>,
) -> Result<Vec<u16>> {
    let (session, _key_file) =
        connect(host, port, username, auth_type, private_key_pem, password).await?;

    // ss 优先，fallback netstat
    let cmd = "ss -tlunH 2>/dev/null | awk '{print $5}' | grep -oE '[0-9]+$' | sort -un; \
               netstat -tlun 2>/dev/null | awk 'NR>2{print $4}' | grep -oE '[0-9]+$' | sort -un";
    let (out, _) = exec(&session, cmd).await?;
    session.close().await.ok();

    let mut ports: Vec<u16> = out
        .lines()
        .filter_map(|l| l.trim().parse::<u16>().ok())
        .filter(|&p| p > 0)
        .collect();
    ports.sort_unstable();
    ports.dedup();
    Ok(ports)
}

/// 获取服务器网卡列表
pub async fn get_interfaces(
    host: &str,
    port: u16,
    username: &str,
    auth_type: &str,
    private_key_pem: Option<&str>,
    password: Option<&str>,
) -> Result<Vec<NetworkInterface>> {
    let (session, _key_file) =
        connect(host, port, username, auth_type, private_key_pem, password).await?;

    let (iface_out, _) = exec(
        &session,
        "ip link show 2>/dev/null | grep -E '^[0-9]+:' | awk -F': ' '{print $2}' | awk '{print $1}' | tr -d '@'",
    )
    .await?;

    let interfaces: Vec<NetworkInterface> = iface_out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|name| NetworkInterface {
            name: name.trim().to_string(),
            description: None,
        })
        .collect();

    session.close().await.ok();
    Ok(interfaces)
}
