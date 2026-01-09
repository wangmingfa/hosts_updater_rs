//! hosts 文件管理模块
//!
//! 提供 hosts 文件的读取、写入、备份和管理功能。

use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

/// hosts 文件标记常量
pub const START_MARKER: &str = "# >>> hosts_updater_rs START >>>";
pub const END_MARKER: &str = "# <<< hosts_updater_rs END <<<";

/// 获取系统 hosts 文件路径
#[cfg(target_os = "windows")]
pub fn get_hosts_path() -> PathBuf {
    PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
}

/// 获取系统 hosts 文件路径
#[cfg(not(target_os = "windows"))]
pub fn get_hosts_path() -> PathBuf {
    PathBuf::from("/etc/hosts")
}

/// 备份 hosts 文件
pub fn backup_hosts(backup_path: &Option<String>) -> Result<String> {
    let hosts_path = get_hosts_path();
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();

    let backup_file_path = match backup_path {
        Some(path) => PathBuf::from(path),
        None => {
            let mut path = PathBuf::from("./backup");
            if !path.exists() {
                fs::create_dir_all(&path)?;
            }
            path.push(format!("hosts.backup.{}", timestamp));
            path
        }
    };

    if hosts_path.exists() {
        fs::copy(&hosts_path, &backup_file_path)
            .with_context(|| format!("备份 hosts 文件失败: {:?}", backup_file_path))?;
    }

    Ok(backup_file_path.to_string_lossy().to_string())
}

/// 读取 hosts 文件内容
pub fn read_hosts_content() -> Result<String> {
    let hosts_path = get_hosts_path();

    if !hosts_path.exists() {
        return Ok(String::new());
    }

    fs::read_to_string(&hosts_path)
        .with_context(|| format!("读取 hosts 文件失败: {:?}", hosts_path))
}

/// 检查是否以管理员权限运行
pub fn check_admin_permission() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Windows 下检查是否以管理员身份运行
        use std::os::windows::process::CommandExt;
        // 尝试以只读方式打开文件来检查权限
        match File::open("C:\\Windows\\System32\\drivers\\etc\\hosts") {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    #[cfg(target_os = "linux")]
    {
        std::fs::metadata("/etc/hosts")
            .and_then(|m| Ok(m.permissions().readonly()))
            .is_err()
    }

    #[cfg(target_os = "macos")]
    {
        std::fs::metadata("/etc/hosts")
            .and_then(|m| Ok(m.permissions().readonly()))
            .is_err()
    }

    #[cfg(target_os = "freebsd")]
    {
        std::fs::metadata("/etc/hosts")
            .and_then(|m| Ok(m.permissions().readonly()))
            .is_err()
    }

    #[cfg(not(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd"
    )))]
    {
        false
    }
}

/// 写入 hosts 文件
///
/// 格式：
/// ```text
/// # >>> hosts_updater_rs START >>>
/// # 此区域由 hosts_updater_rs 自动管理，请勿手动修改
/// # 最后更新: 2024-01-15 10:30:00
///
/// # Source: https://example.com/hosts1
/// 127.0.0.1 localhost
/// 192.168.1.100 example.com
///
/// # Source: https://example.com/hosts2
/// 192.168.1.101 api.example.com
///
/// # <<< hosts_updater_rs END <<<
/// ```
pub fn write_hosts(sources: &[(String, String)], last_update: &str) -> Result<()> {
    let hosts_path = get_hosts_path();

    // 读取现有内容
    let existing_content = read_hosts_content()?;

    // 移除旧的自动管理区域
    let cleaned_content = remove_auto_managed_section(&existing_content);

    // 构建新的自动管理区域
    let auto_section = build_auto_section(sources, last_update);

    // 组合内容
    let new_content = if cleaned_content.trim().is_empty() {
        auto_section
    } else {
        format!("{}\n\n{}", cleaned_content.trim_end(), auto_section)
    };

    // 写入文件
    let mut file = File::create(&hosts_path)
        .with_context(|| format!("创建 hosts 文件失败: {:?}", hosts_path))?;

    file.write_all(new_content.as_bytes())
        .with_context(|| format!("写入 hosts 文件失败: {:?}", hosts_path))?;

    Ok(())
}

/// 移除自动管理区域
fn remove_auto_managed_section(content: &str) -> String {
    let mut result = String::new();
    let mut in_auto_section = false;
    let mut found_start = false;

    for line in content.lines() {
        if line.trim() == START_MARKER {
            in_auto_section = true;
            found_start = true;
            continue;
        }

        if line.trim() == END_MARKER {
            in_auto_section = false;
            continue;
        }

        if !in_auto_section {
            result.push_str(line);
            result.push('\n');
        }
    }

    // 如果没有找到标记，返回原内容
    if !found_start {
        content.to_string()
    } else {
        result.trim_end().to_string()
    }
}

/// 构建自动管理区域
fn build_auto_section(sources: &[(String, String)], last_update: &str) -> String {
    let mut section = String::new();

    section.push_str(START_MARKER);
    section.push('\n');
    section.push_str("# 此区域由 hosts_updater_rs 自动管理，请勿手动修改");
    section.push('\n');
    section.push_str("# 最后更新: ");
    section.push_str(last_update);
    section.push_str("\n\n");

    for (url, content) in sources {
        section.push_str("# Source: ");
        section.push_str(url);
        section.push('\n');
        section.push_str(content.trim());
        section.push_str("\n\n");
    }

    section.push_str(END_MARKER);
    section.push('\n');

    section
}
