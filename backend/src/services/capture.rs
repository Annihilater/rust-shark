use crate::db::DbPool;
use crate::models::capture::CaptureTask;
use crate::models::server::Server;
use crate::services::ssh;
use crate::state::CaptureRegistry;
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
    capture_pids: CaptureRegistry,
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

    // ── 启动前：kill 该服务器上所有残留的 tcpdump 进程 ──────────────────
    let (kill_out, _) = ssh::exec(
        &session,
        "pids=$(pgrep -x tcpdump 2>/dev/null); [ -n \"$pids\" ] && kill $pids && echo \"killed: $pids\" || echo 'no stale tcpdump'",
    ).await?;
    info!("清理残留 tcpdump: {}", kill_out.trim());
    // 稍等进程退出
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

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

    // 后台启动 tcpdump，立即拿到 PID，供停止时使用
    // 用 setsid 让子进程脱离 SSH session，这样 session 断开也不会死
    let bg_cmd = if let Some(dur) = task.duration {
        format!(
            "sudo timeout {} {} & echo $!",
            dur, tcpdump_cmd
        )
    } else {
        format!("sudo {} & echo $!", tcpdump_cmd)
    };

    info!("执行抓包命令: {}", bg_cmd);

    // 更新状态为 running
    sqlx::query("UPDATE capture_tasks SET status = 'running' WHERE id = ?")
        .bind(&task.id)
        .execute(&pool)
        .await?;

    // 后台启动并取得 PID
    let (pid_out, _) = ssh::exec(&session, &bg_cmd).await?;
    let remote_pid: u32 = pid_out.trim().parse().unwrap_or(0);
    info!("tcpdump 后台 PID: {}", remote_pid);

    // 注册到全局 registry
    if remote_pid > 0 {
        capture_pids.lock().await.insert(
            task.id.clone(),
            (server.id.clone(), remote_pid),
        );
    }

    // 轮询等待进程结束（每 2 秒检查一次）
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // 检查数据库是否被 stop 接口标记为 cancelled
        let status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM capture_tasks WHERE id = ?",
        )
        .bind(&task.id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        if status.as_deref() == Some("cancelled") {
            // 用户点了停止：kill 进程
            if remote_pid > 0 {
                let kill_cmd = format!("kill {} 2>/dev/null; pkill -P {} 2>/dev/null; true", remote_pid, remote_pid);
                ssh::exec(&session, &kill_cmd).await.ok();
                info!("已 kill tcpdump PID {}", remote_pid);
            }
            break;
        }

        // 检查进程是否还活着
        let (_, alive_code) = ssh::exec(
            &session,
            &format!("kill -0 {} 2>/dev/null", remote_pid),
        ).await.unwrap_or(("".to_string(), 1));

        if alive_code != 0 {
            // 进程已自然结束
            info!("tcpdump PID {} 已结束", remote_pid);
            break;
        }
    }

    // 从 registry 移除
    capture_pids.lock().await.remove(&task.id);

    // 检查当前状态（可能已被标记为 cancelled）
    let current_status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM capture_tasks WHERE id = ?",
    )
    .bind(&task.id)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    if current_status.as_deref() == Some("cancelled") {
        // 即便取消，也尝试下载已有的部分数据
        info!("任务已取消，尝试下载已有数据: {}", task.id);
    }

    // 通过 base64 下载 pcap 文件
    let (file_data_b64, code) = ssh::exec(&session, &format!("base64 {} 2>/dev/null", remote_file)).await?;
    if code != 0 || file_data_b64.trim().is_empty() {
        // 没有文件（比如刚启动就被 cancel）
        sqlx::query(
            "UPDATE capture_tasks SET status = 'cancelled', finished_at = datetime('now') WHERE id = ? AND status = 'running'",
        )
        .bind(&task.id)
        .execute(&pool)
        .await?;
        ssh::exec(&session, &format!("rm -f {}", remote_file)).await.ok();
        session.close().await.ok();
        return Err(anyhow::anyhow!("无抓包数据"));
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
