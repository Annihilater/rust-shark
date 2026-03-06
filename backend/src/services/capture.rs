use crate::db::DbPool;
use crate::models::capture::CaptureTask;
use crate::models::server::Server;
use crate::services::ssh;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::{error, info, warn};

/// tcpdump 二进制（内嵌，用于上传到目标服务器）
/// CARGO_MANIFEST_DIR = backend/，所以 ../assets/ = 项目根目录
#[cfg(target_arch = "x86_64")]
static TCPDUMP_BINARY: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/binaries/tcpdump-linux-amd64"));

#[cfg(target_arch = "aarch64")]
static TCPDUMP_BINARY: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/binaries/tcpdump-linux-arm64"));

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
static TCPDUMP_BINARY: &[u8] = &[0u8];

/// 确保目标服务器上有 tcpdump，返回可用路径
pub async fn ensure_tcpdump(
    session: &openssh::Session,
    _arch: &str,
) -> Result<String> {
    // 先检查是否已安装
    let (stdout, code) = ssh::exec(session, "which tcpdump 2>/dev/null").await?;
    if code == 0 && !stdout.trim().is_empty() {
        info!("tcpdump 已存在: {}", stdout.trim());
        return Ok(stdout.trim().to_string());
    }

    info!("tcpdump 未找到，尝试自动安装...");

    // 检测包管理器并安装
    let install_cmds = [
        "sudo apt-get install -y tcpdump 2>/dev/null && which tcpdump",
        "sudo apt install -y tcpdump 2>/dev/null && which tcpdump",
        "sudo yum install -y tcpdump 2>/dev/null && which tcpdump",
        "sudo dnf install -y tcpdump 2>/dev/null && which tcpdump",
        "sudo apk add tcpdump 2>/dev/null && which tcpdump",
    ];

    for cmd in &install_cmds {
        let (out, code) = ssh::exec(session, cmd).await?;
        if code == 0 && !out.trim().is_empty() {
            info!("通过包管理器安装 tcpdump 成功: {}", out.trim());
            return Ok(out.trim().to_string());
        }
    }

    // 包管理器安装失败，检查内嵌二进制是否有效
    if TCPDUMP_BINARY.len() <= 1 {
        anyhow::bail!("tcpdump 安装失败，且当前编译的内嵌二进制为占位符，请在正式构建中嵌入真实二进制");
    }

    warn!("包管理器安装失败，上传内嵌 tcpdump 二进制...");

    // 根据远端架构选择合适的二进制
    // （实际生产中 include_bytes! 已根据编译目标选择好了）
    let remote_path = "/tmp/.tcpdump-rs";
    upload_binary(session, TCPDUMP_BINARY, remote_path).await?;

    let (_, code) = ssh::exec(session, &format!("sudo chmod +x {} && {} --version > /dev/null 2>&1", remote_path, remote_path)).await?;
    if code != 0 {
        anyhow::bail!("上传的 tcpdump 二进制无法执行");
    }

    info!("内嵌 tcpdump 二进制上传成功: {}", remote_path);
    Ok(remote_path.to_string())
}

/// 通过 base64 编码上传二进制文件到远端
async fn upload_binary(
    session: &openssh::Session,
    data: &[u8],
    remote_path: &str,
) -> Result<()> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

    // 分块上传（避免命令行参数过长）
    let chunk_size = 32768; // 32KB chunks
    let encoded = BASE64.encode(data);

    // 先清空目标文件
    ssh::exec(session, &format!("echo -n '' > {}", remote_path)).await?;

    // 分块写入
    for chunk in encoded.as_bytes().chunks(chunk_size) {
        let chunk_str = String::from_utf8_lossy(chunk);
        let cmd = format!("echo -n '{}' >> {}.b64", chunk_str, remote_path);
        ssh::exec(session, &cmd).await?;
    }

    // base64 解码
    let cmd = format!("base64 -d {}.b64 > {} && rm {}.b64", remote_path, remote_path, remote_path);
    let (_, code) = ssh::exec(session, &cmd).await?;
    if code != 0 {
        anyhow::bail!("base64 解码失败");
    }

    Ok(())
}

/// 执行抓包任务
pub async fn run_capture(
    task: &CaptureTask,
    server: &Server,
    private_key_pem: Option<&str>,
    password: Option<&str>,
    data_dir: &str,
    pool: DbPool,
) -> Result<PathBuf> {
    let (session, _key_file) = ssh::connect(
        &server.host,
        server.port as u16,
        &server.username,
        &server.auth_type,
        private_key_pem,
        password,
    )
    .await?;

    // 检测远端架构
    let (arch, _) = ssh::exec(&session, "uname -m").await?;
    let arch = arch.trim().to_string();

    // 确保 tcpdump 存在
    let tcpdump_path = ensure_tcpdump(&session, &arch).await?;

    // 构建 tcpdump 命令
    let remote_file = format!("/tmp/capture-{}.pcap", task.id);
    let filter_part = task
        .filter
        .as_deref()
        .filter(|f| !f.is_empty())
        .map(|f| format!(" '{}'", f))
        .unwrap_or_default();
    let count_part = task
        .packet_limit
        .map(|c| format!(" -c {}", c))
        .unwrap_or_default();

    let tcpdump_cmd = format!(
        "{} -i {} -w {}{}{}",
        tcpdump_path, task.interface, remote_file, filter_part, count_part
    );

    let cmd = if let Some(dur) = task.duration {
        format!("sudo timeout {} {}", dur, tcpdump_cmd)
    } else {
        format!("sudo {}", tcpdump_cmd)
    };

    info!("执行抓包命令: {}", cmd);

    // 更新状态为 running
    sqlx::query("UPDATE capture_tasks SET status = 'running' WHERE id = ?")
        .bind(&task.id)
        .execute(&pool)
        .await?;

    // 执行抓包（同步等待完成）
    let (stderr, exit_code) = {
        let output = session
            .command("sh")
            .arg("-c")
            .arg(&cmd)
            .output()
            .await?;
        (
            String::from_utf8_lossy(&output.stderr).to_string(),
            output.status.code().unwrap_or(-1),
        )
    };

    // exit_code 124 = timeout 正常结束
    if exit_code != 0 && exit_code != 124 {
        let err_msg = format!("抓包失败 (exit={}): {}", exit_code, stderr.trim());
        error!("{}", err_msg);
        sqlx::query(
            "UPDATE capture_tasks SET status = 'failed', error_msg = ?, finished_at = datetime('now') WHERE id = ?",
        )
        .bind(&err_msg)
        .bind(&task.id)
        .execute(&pool)
        .await?;
        anyhow::bail!("{}", err_msg);
    }

    // 通过 base64 下载 pcap 文件
    let (file_data_b64, code) = ssh::exec(&session, &format!("base64 {}", remote_file)).await?;
    if code != 0 {
        anyhow::bail!("读取远端抓包文件失败");
    }

    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    let file_data = BASE64
        .decode(file_data_b64.trim())
        .context("base64解码抓包文件失败")?;

    // 保存到本地
    let local_dir = PathBuf::from(data_dir)
        .join("captures")
        .join(&task.user_id);
    tokio::fs::create_dir_all(&local_dir).await?;
    let local_path = local_dir.join(format!("{}.pcap", task.id));
    tokio::fs::write(&local_path, &file_data).await?;
    let file_size = file_data.len() as i64;

    // 清理远端临时文件
    ssh::exec(&session, &format!("rm -f {}", remote_file))
        .await
        .ok();

    // 更新任务状态
    sqlx::query(
        "UPDATE capture_tasks SET status = 'done', file_path = ?, file_size = ?, finished_at = datetime('now') WHERE id = ?",
    )
    .bind(local_path.to_str().unwrap_or(""))
    .bind(file_size)
    .bind(&task.id)
    .execute(&pool)
    .await?;

    session.close().await.ok();
    info!("抓包完成: {} ({} bytes)", task.id, file_size);

    Ok(local_path)
}
