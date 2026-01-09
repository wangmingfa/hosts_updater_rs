//! hosts_updater_rs - Hosts 文件自动更新工具
//!
//! 一个用 Rust 编写的 Hosts 文件自动更新工具，定时从配置源获取 hosts 规则
//! 并写入系统 hosts 文件，帮助实现域名访问加速。

mod config;
mod fetcher;
mod hosts;
mod scheduler;

use anyhow::{Context, Result};
use config::{load_config, validate_config, Config};
use fetcher::fetch_all_hosts;
use hosts::{
    backup_hosts, check_admin_permission, get_hosts_path, read_hosts_content, write_hosts,
};
use scheduler::Scheduler;
use std::boxed::Box;
use std::future::Future;
use std::pin::Pin;
use tracing::{error, info, warn};

/// 程序入口
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    info!("hosts_updater_rs 启动");

    // 检查管理员权限
    if !check_admin_permission() {
        warn!("程序未以管理员权限运行，可能无法修改系统 hosts 文件");
        #[cfg(target_os = "windows")]
        {
            println!("警告: 程序需要管理员权限才能修改系统 hosts 文件");
            println!("请右键点击程序，选择 '以管理员身份运行'");
        }
        #[cfg(not(target_os = "windows"))]
        {
            println!("警告: 程序需要 root 权限才能修改系统 hosts 文件");
            println!("请使用 sudo 运行: sudo {} ", std::env::current_exe()?.display());
        }
    }

    // 加载配置
    let config = load_config().context("加载配置文件失败")?;
    validate_config(&config).context("配置验证失败")?;

    info!("配置加载成功，更新间隔: {} 小时", config.update_interval_hours);
    info!("数据源数量: {}", config.hosts_sources.len());

    // 创建更新任务
    let update_task = create_update_task(config.clone());

    // 启动定时任务
    let scheduler = Scheduler::new(config.update_interval_hours);
    scheduler.start(update_task).await;

    Ok(())
}

/// 创建更新任务闭包
fn create_update_task(config: Config) -> impl FnMut() -> Pin<Box<dyn Future<Output = ()> + Send>> {
    move || {
        let config = config.clone();
        Box::pin(async move {
            if let Err(e) = run_update(&config).await {
                error!("更新 hosts 失败: {:?}", e);
            }
        })
    }
}

/// 执行一次更新
async fn run_update(config: &Config) -> Result<()> {
    info!("开始更新 hosts 文件...");

    let hosts_path = get_hosts_path();
    info!("目标 hosts 文件: {:?}", hosts_path);

    // 备份现有 hosts
    if config.backup_before_update {
        let backup_path = backup_hosts(&config.backup_path)?;
        info!("已备份 hosts 文件到: {}", backup_path);
    }

    // 获取当前 hosts 内容
    let current_content = read_hosts_content()?;
    info!("当前 hosts 文件大小: {} 字节", current_content.len());

    // 从所有数据源获取 hosts 内容
    info!("开始从 {} 个数据源获取 hosts...", config.hosts_sources.len());
    let sources_content = fetch_all_hosts(&config.hosts_sources)?;
    info!("成功获取 {} 个数据源的内容", sources_content.len());

    // 生成最后更新时间
    let last_update = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 写入 hosts 文件
    write_hosts(&sources_content, &last_update)?;
    info!("hosts 文件更新成功");

    Ok(())
}
