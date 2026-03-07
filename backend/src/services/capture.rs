use crate::db::DbPool;
use crate::models::capture::CaptureTask;
use crate::models::server::Server;
use crate::services::ssh;
use crate::state::CaptureRegistry;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing::info;

/// 确保目标服务器上有 tcpdump，返回可用路径
/// 只检查 which tcpdump；若未找到直接返回错误，不尝试安装
pub async fn ensure_tcpdump(
    session: &openssh::Session,
    _arch: &str,
) -> Result<String> {
    let (stdout, code) = ssh::exec(session, "which tcpdump 2>/dev/null").await?;
    if code == 0 && !stdout.trim().is_empty() {
        info!("tcpdump 已存在: {}", stdout.trim());
        return Ok(stdout.trim().to_string());
    }
    anyhow::bail!("目标服务器上未找到 tcpdump，请先安装后重试");
}

/// 更新任务的 log_msg 字段
async fn update_log(pool: &DbPool, task_id: &str, msg: &str) {
    sqlx::query("UPDATE capture_tasks SET log_msg = ? WHERE id = ?")
        .bind(msg)
        .bind(task_id)
        .execute(pool)
        .await
        .ok();
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
    update_log(&pool, &task.id, "检查残留进程...").await;

    let (kill_out, _) = ssh::exec(
        &session,
        "pids=$(pgrep -x tcpdump 2>/dev/null); [ -n \"$pids\" ] && kill $pids && echo \"killed: $pids\" || echo 'no stale tcpdump'",
    ).await?;
    info!("清理残留 tcpdump: {}", kill_out.trim());

    let kill_log = if kill_out.trim() == "no stale tcpdump" {
        "无残留进程".to_string()
    } else {
        format!("已清理 {}", kill_out.trim())
    };
    update_log(&pool, &task.id, &kill_log).await;

    // 稍等进程退出
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // 检测远端架构
    let (arch, _) = ssh::exec(&session, "uname -m").await?;
    let arch = arch.trim().to_string();

    // 确保 tcpdump 存在
    let tcpdump_path = ensure_tcpdump(&session, &arch).await?;
    let tcpdump_log = format!("tcpdump 路径: {}", tcpdump_path);
    update_log(&pool, &task.id, &tcpdump_log).await;

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
    let bg_cmd = if let Some(dur) = task.duration {
        format!(
            "sudo timeout {} {} & echo $!",
            dur, tcpdump_cmd
        )
    } else {
        format!("sudo {} & echo $!", tcpdump_cmd)
    };

    let cmd_log = format!("执行命令: {}", bg_cmd);
    info!("{}", cmd_log);
    update_log(&pool, &task.id, &cmd_log).await;

    // 更新状态为 running
    sqlx::query("UPDATE capture_tasks SET status = 'running' WHERE id = ?")
        .bind(&task.id)
        .execute(&pool)
        .await?;

    // 后台启动并取得 PID
    let (pid_out, _) = ssh::exec(&session, &bg_cmd).await?;
    let remote_pid: u32 = pid_out.trim().parse().unwrap_or(0);
    info!("tcpdump 后台 PID: {}", remote_pid);

    let pid_log = format!("tcpdump 已启动 (PID: {})", remote_pid);
    update_log(&pool, &task.id, &pid_log).await;

    // 注册到全局 registry
    if remote_pid > 0 {
        capture_pids.lock().await.insert(
            task.id.clone(),
            (server.id.clone(), remote_pid),
        );
    }

    // 轮询等待进程结束（每 2 秒检查一次）
    let mut poll_counter: u32 = 0;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        poll_counter += 1;

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
            update_log(&pool, &task.id, "用户已停止").await;
            break;
        }

        // 每 5 次轮询（≈10 秒）更新一次包计数日志
        if poll_counter % 5 == 0 {
            let count_cmd = format!(
                "tcpdump -r {} --count 2>/dev/null | tail -1",
                remote_file
            );
            if let Ok((count_out, _)) = ssh::exec(&session, &count_cmd).await {
                let count_str = count_out.trim().to_string();
                if !count_str.is_empty() {
                    let progress_log = format!("tcpdump 已启动 (PID: {})\n已捕获: {}", remote_pid, count_str);
                    update_log(&pool, &task.id, &progress_log).await;
                }
            }
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

    update_log(&pool, &task.id, "抓包完成，正在下载数据...").await;

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
