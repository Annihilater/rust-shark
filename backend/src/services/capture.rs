use crate::db::DbPool;
use crate::models::capture::CaptureTask;
use crate::models::server::Server;
use crate::services::ssh;
use crate::state::CaptureRegistry;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::info;

/// 确保目标服务器上有 tcpdump，返回可用路径
/// 只检查 which tcpdump；若未找到直接返回错误，不尝试安装
pub async fn ensure_tcpdump(session: &openssh::Session, _arch: &str) -> Result<String> {
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

/// 通过 scp 从远端下载文件到本地
/// 使用系统 scp 命令，支持任意大小文件，不受内存限制
#[allow(clippy::too_many_arguments)]
async fn download_via_scp(
    host: &str,
    port: u16,
    username: &str,
    auth_type: &str,
    private_key_pem: Option<&str>,
    key_file_path: Option<&Path>,
    remote_path: &str,
    local_path: &Path,
) -> Result<()> {
    // 写临时密钥文件（如果没有已有的）
    let _tmp_key;
    let key_path_for_scp: Option<&Path> = if auth_type == "key" {
        if let Some(p) = key_file_path {
            Some(p)
        } else if let Some(pem) = private_key_pem {
            let mut tmp = tempfile::NamedTempFile::new()?;
            use std::io::Write;
            tmp.write_all(pem.as_bytes())?;
            tmp.flush()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600))?;
            }
            _tmp_key = tmp;
            Some(_tmp_key.path())
        } else {
            None
        }
    } else {
        None
    };

    let mut cmd = tokio::process::Command::new("scp");
    cmd.arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=30")
        .arg("-P")
        .arg(port.to_string());

    if let Some(key_path) = key_path_for_scp {
        cmd.arg("-i").arg(key_path);
    }

    let src = format!("{}@{}:{}", username, host, remote_path);
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        cmd.arg(&src).arg(local_path).output(),
    )
    .await
    .context("scp 超时（120s）")?
    .context("scp 命令执行失败")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("scp 失败: {}", stderr.trim());
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
    // 先写初始日志让前端可见，再做 SSH 连接（连接可能耗时）
    update_log(
        &pool,
        &task.id,
        &format!("正在 SSH 连接 {}:{}...", server.host, server.port),
    )
    .await;

    let connect_result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        ssh::connect(
            &server.host,
            server.port as u16,
            &server.username,
            &server.auth_type,
            private_key_pem,
            password,
        ),
    )
    .await;

    let (session, key_file) = match connect_result {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            let msg = format!("❌ SSH 连接失败: {:#}", e);
            update_log(&pool, &task.id, &msg).await;
            return Err(anyhow::anyhow!("{}", msg));
        }
        Err(_) => {
            let msg = format!(
                "❌ SSH 连接超时（30s），请检查服务器 {}:{} 是否可达",
                server.host, server.port
            );
            update_log(&pool, &task.id, &msg).await;
            return Err(anyhow::anyhow!("{}", msg));
        }
    };

    update_log(&pool, &task.id, "SSH 连接成功").await;

    // SSH 连接成功后立即将状态改为 running，让前端可见日志面板
    sqlx::query("UPDATE capture_tasks SET status = 'running' WHERE id = ?")
        .bind(&task.id)
        .execute(&pool)
        .await?;

    // ── 启动前：检查残留进程，仅提示，不强制 kill ───────────────────────
    update_log(&pool, &task.id, "检查远端环境...").await;

    let (stale_out, _) = ssh::exec(
        &session,
        "pgrep -x tcpdump 2>/dev/null | tr '\\n' ',' | sed 's/,$//'",
    )
    .await?;
    let stale_pids = stale_out.trim().to_string();
    if !stale_pids.is_empty() {
        let warn = format!("⚠ 警告：服务器上已有 tcpdump 进程 (PID: {})，未强制终止。\n  如遇权限或资源冲突，请先手动停止旧任务再重试。", stale_pids);
        info!("{}", warn);
        update_log(&pool, &task.id, &warn).await;
    } else {
        update_log(&pool, &task.id, "无残留进程").await;
    }

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
        format!("sudo timeout {} {} & echo $!", dur, tcpdump_cmd)
    } else {
        format!("sudo {} & echo $!", tcpdump_cmd)
    };

    let cmd_log = format!("执行命令: {}", bg_cmd);
    info!("{}", cmd_log);
    update_log(&pool, &task.id, &cmd_log).await;

    // (status 已在 SSH 连接成功时设为 running，无需重复更新)

    // 后台启动并取得 PID
    let (pid_out, pid_exit) = ssh::exec(&session, &bg_cmd).await?;
    let remote_pid: u32 = pid_out
        .trim()
        .lines()
        .last()
        .unwrap_or("")
        .parse()
        .unwrap_or(0);
    info!("tcpdump 后台 PID: {} (exit: {})", remote_pid, pid_exit);

    if remote_pid == 0 || pid_exit != 0 {
        // 启动失败：将任务标记为 failed，写入错误日志，供用户排查
        let err_log = format!(
            "❌ tcpdump 启动失败 (exit code: {})\n输出: {}\n\n可能原因：\n  • 服务器上已有 tcpdump 占用接口（请先停止旧任务）\n  • 缺少 sudo 权限\n  • 网卡名称 \"{}\" 不存在\n请根据日志排查后重试。",
            pid_exit,
            pid_out.trim(),
            task.interface,
        );
        update_log(&pool, &task.id, &err_log).await;
        sqlx::query(
            "UPDATE capture_tasks SET status = 'failed', finished_at = datetime('now') WHERE id = ?",
        )
        .bind(&task.id)
        .execute(&pool)
        .await?;
        session.close().await.ok();
        return Err(anyhow::anyhow!("tcpdump 启动失败，详见任务日志"));
    }

    let pid_log = format!("tcpdump 已启动 (PID: {})", remote_pid);
    update_log(&pool, &task.id, &pid_log).await;

    // 注册到全局 registry
    if remote_pid > 0 {
        capture_pids
            .lock()
            .await
            .insert(task.id.clone(), (server.id.clone(), remote_pid));
    }

    // 轮询等待进程结束（每 2 秒检查一次）
    let mut poll_counter: u32 = 0;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        poll_counter += 1;

        // 检查数据库是否被 stop 接口标记为 cancelled
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM capture_tasks WHERE id = ?")
                .bind(&task.id)
                .fetch_optional(&pool)
                .await
                .unwrap_or(None);

        if status.as_deref() == Some("cancelled") {
            // 用户点了停止：kill 进程
            if remote_pid > 0 {
                let kill_cmd = format!(
                    "kill {} 2>/dev/null; pkill -P {} 2>/dev/null; true",
                    remote_pid, remote_pid
                );
                ssh::exec(&session, &kill_cmd).await.ok();
                info!("已 kill tcpdump PID {}", remote_pid);
            }
            update_log(&pool, &task.id, "用户已停止").await;
            break;
        }

        // 每 5 次轮询（≈10 秒）更新一次包计数日志
        if poll_counter.is_multiple_of(5) {
            let count_cmd = format!("tcpdump -r {} --count 2>/dev/null | tail -1", remote_file);
            if let Ok((count_out, _)) = ssh::exec(&session, &count_cmd).await {
                let count_str = count_out.trim().to_string();
                if !count_str.is_empty() {
                    let progress_log = format!(
                        "tcpdump 已启动 (PID: {})\n已捕获: {}",
                        remote_pid, count_str
                    );
                    update_log(&pool, &task.id, &progress_log).await;
                }
            }
        }

        // 检查进程是否还活着
        let (_, alive_code) = ssh::exec(&session, &format!("kill -0 {} 2>/dev/null", remote_pid))
            .await
            .unwrap_or(("".to_string(), 1));

        if alive_code != 0 {
            // 进程已自然结束
            info!("tcpdump PID {} 已结束", remote_pid);
            break;
        }
    }

    // 从 registry 移除
    capture_pids.lock().await.remove(&task.id);

    // 检查当前状态（可能已被标记为 cancelled）
    let current_status: Option<String> =
        sqlx::query_scalar("SELECT status FROM capture_tasks WHERE id = ?")
            .bind(&task.id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

    if current_status.as_deref() == Some("cancelled") {
        // 即便取消，也尝试下载已有的部分数据
        info!("任务已取消，尝试下载已有数据: {}", task.id);
    }

    update_log(&pool, &task.id, "抓包完成，正在下载数据...").await;

    // 保存到本地（使用绝对路径，确保 sharkd 能找到文件）
    let base_dir = if std::path::Path::new(data_dir).is_absolute() {
        PathBuf::from(data_dir)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(data_dir)
    };
    let local_dir = base_dir.join("captures").join(&task.user_id);
    tokio::fs::create_dir_all(&local_dir).await?;
    let local_path = local_dir.join(format!("{}.pcap", task.id));

    // 通过 scp 下载 pcap 文件（比 base64 更可靠，支持大文件）
    let scp_result = download_via_scp(
        &server.host,
        server.port as u16,
        &server.username,
        &server.auth_type,
        private_key_pem,
        key_file.as_ref().map(|f| f.path()),
        &remote_file,
        &local_path,
    )
    .await;

    if let Err(e) = scp_result {
        info!("scp 失败，回退到 base64: {}", e);
        // fallback: base64
        let (file_data_b64, code) =
            ssh::exec(&session, &format!("base64 {} 2>/dev/null", remote_file)).await?;
        if code != 0 || file_data_b64.trim().is_empty() {
            sqlx::query(
                "UPDATE capture_tasks SET status = 'cancelled', finished_at = datetime('now') WHERE id = ? AND status = 'running'",
            )
            .bind(&task.id)
            .execute(&pool)
            .await?;
            ssh::exec(&session, &format!("rm -f {}", remote_file))
                .await
                .ok();
            session.close().await.ok();
            return Err(anyhow::anyhow!("无抓包数据"));
        }
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
        let file_data = BASE64
            .decode(file_data_b64.trim())
            .context("base64解码抓包文件失败")?;
        tokio::fs::write(&local_path, &file_data).await?;
    }

    // 检查文件是否存在且非空
    let file_size = match tokio::fs::metadata(&local_path).await {
        Ok(m) if m.len() > 0 => m.len() as i64,
        _ => {
            sqlx::query(
                "UPDATE capture_tasks SET status = 'cancelled', finished_at = datetime('now') WHERE id = ? AND status = 'running'",
            )
            .bind(&task.id)
            .execute(&pool)
            .await?;
            ssh::exec(&session, &format!("rm -f {}", remote_file))
                .await
                .ok();
            session.close().await.ok();
            return Err(anyhow::anyhow!("无抓包数据"));
        }
    };

    // 清理远端临时文件
    ssh::exec(&session, &format!("rm -f {}", remote_file))
        .await
        .ok();

    // 更新任务状态
    let done_log = format!("抓包完成，共 {} 字节", file_size);
    update_log(&pool, &task.id, &done_log).await;
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
