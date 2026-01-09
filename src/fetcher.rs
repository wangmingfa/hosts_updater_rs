//! 网络获取模块
//!
//! 提供从 URL 获取 hosts 内容的功能。

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::time::Duration;

/// HTTP 客户端超时配置
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 从 URL 获取 hosts 内容
///
/// 返回纯文本格式的 hosts 内容，可直接追加到系统 hosts 文件。
pub fn fetch_hosts_content(url: &str) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
        .build()
        .context("创建 HTTP 客户端失败")?;

    let response = client
        .get(url)
        .send()
        .with_context(|| format!("请求 URL 失败: {}", url))?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "请求失败，HTTP 状态码: {}",
            response.status()
        ));
    }

    let content = response
        .text()
        .with_context(|| format!("读取响应内容失败: {}", url))?;

    // 验证内容格式
    validate_hosts_content(&content, url)?;

    Ok(content)
}

/// 验证 hosts 内容格式
fn validate_hosts_content(content: &str, url: &str) -> Result<()> {
    if content.trim().is_empty() {
        return Err(anyhow::anyhow!("URL 返回内容为空: {}", url));
    }

    // 检查是否包含非法字符（控制字符等）
    for (i, c) in content.chars().enumerate() {
        if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
            return Err(anyhow::anyhow!(
                "URL 返回内容包含非法控制字符 (位置 {}): {}",
                i,
                url
            ));
        }
    }

    // 逐行检查 hosts 格式
    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // 跳过空行和注释行
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // 检查是否为有效的 hosts 格式: IP + 域名
        if let Err(e) = validate_hosts_line(line, line_num + 1, url) {
            return Err(e);
        }
    }

    Ok(())
}

/// 验证单行 hosts 配置格式
fn validate_hosts_line(line: &str, line_num: usize, url: &str) -> Result<()> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 2 {
        return Err(anyhow::anyhow!(
            "第 {} 行格式无效，缺少 IP 或域名: {} (来源: {})",
            line_num,
            line,
            url
        ));
    }

    let ip = parts[0];

    // 验证 IP 地址格式
    if !is_valid_ip(ip) {
        return Err(anyhow::anyhow!(
            "第 {} 行 IP 地址格式无效: {} (来源: {})",
            line_num,
            ip,
            url
        ));
    }

    // 验证每个域名格式
    for domain in &parts[1..] {
        if !is_valid_domain(domain) {
            return Err(anyhow::anyhow!(
                "第 {} 行域名格式无效: {} (来源: {})",
                line_num,
                domain,
                url
            ));
        }
    }

    Ok(())
}

/// 验证域名格式
fn is_valid_domain(domain: &str) -> bool {
    // 域名不能为空
    if domain.is_empty() {
        return false;
    }

    // 域名长度限制（总长度 253 字符以内）
    if domain.len() > 253 {
        return false;
    }

    // 每段标签长度限制（1-63 字符）
    let labels: Vec<&str> = domain.split('.').collect();
    for label in &labels {
        let label_len = label.len();
        if label_len == 0 || label_len > 63 {
            return false;
        }

        // 标签必须以字母或数字开头和结尾
        let bytes = label.as_bytes();
        let first_char = bytes[0] as char;
        let last_char = bytes[bytes.len() - 1] as char;

        if !first_char.is_alphanumeric() || !last_char.is_alphanumeric() {
            return false;
        }

        // 标签只能包含字母、数字和连字符
        for &byte in bytes {
            let c = byte as char;
            if !c.is_alphanumeric() && c != '-' {
                return false;
            }
        }
    }

    true
}

/// 验证 IP 地址格式（支持 IPv4 和 IPv6）
fn is_valid_ip(ip: &str) -> bool {
    // IPv4 检查
    if ip.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }

    // IPv6 检查（方括号格式）
    if ip.starts_with('[') && ip.ends_with(']') {
        let ipv6 = &ip[1..ip.len() - 1];
        return ipv6.parse::<std::net::Ipv6Addr>().is_ok();
    }

    // 纯 IPv6 检查
    if ip.parse::<std::net::Ipv6Addr>().is_ok() {
        return true;
    }

    false
}

/// 批量获取多个数据源的 hosts 内容
///
/// 返回 (URL, 内容) 元组的向量。
pub fn fetch_all_hosts(sources: &[String]) -> Result<Vec<(String, String)>> {
    let mut results = Vec::new();

    for url in sources {
        match fetch_hosts_content(url) {
            Ok(content) => {
                results.push((url.clone(), content));
                tracing::info!("成功获取 hosts 内容: {}", url);
            }
            Err(e) => {
                tracing::error!("获取 hosts 内容失败: {}, 错误: {}", url, e);
                return Err(e);
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_hosts_content_valid() {
        let content = r#"
# 注释行
127.0.0.1 localhost
192.168.1.100 example.com
"#;

        assert!(validate_hosts_content(content, "https://example.com").is_ok());
    }

    #[test]
    fn test_validate_hosts_content_empty() {
        let content = "";
        assert!(validate_hosts_content(content, "https://example.com").is_err());
    }

    #[test]
    fn test_validate_hosts_content_with_control_chars() {
        let content = "127.0.0.1 localhost\x00";
        assert!(validate_hosts_content(content, "https://example.com").is_err());
    }

    #[test]
    fn test_validate_hosts_line_valid_ipv4() {
        assert!(is_valid_ip("127.0.0.1"));
        assert!(is_valid_ip("192.168.1.100"));
        assert!(is_valid_ip("0.0.0.0"));
    }

    #[test]
    fn test_validate_hosts_line_valid_ipv6() {
        assert!(is_valid_ip("::1"));
        assert!(is_valid_ip("2001:0db8:85a3:0000:0000:8a2e:0370:7334"));
        assert!(is_valid_ip("[::1]"));
    }

    #[test]
    fn test_validate_hosts_line_invalid_ip() {
        assert!(!is_valid_ip("invalid"));
        assert!(!is_valid_ip("256.1.1.1"));
        assert!(!is_valid_ip("abc.def.ghi.jkl"));
    }

    #[test]
    fn test_validate_hosts_content_invalid_line() {
        let content = "127.0.0.1\ninvalid_line_without_ip\n192.168.1.100 example.com";
        assert!(validate_hosts_content(content, "https://example.com").is_err());
    }

    #[test]
    fn test_validate_hosts_content_missing_domain() {
        let content = "127.0.0.1";
        assert!(validate_hosts_content(content, "https://example.com").is_err());
    }

    #[test]
    fn test_is_valid_domain_valid() {
        assert!(is_valid_domain("example.com"));
        assert!(is_valid_domain("sub.example.com"));
        assert!(is_valid_domain("localhost"));
        assert!(is_valid_domain("my-server-123.com"));
        assert!(is_valid_domain("a1b2c3.com"));
    }

    #[test]
    fn test_is_valid_domain_invalid() {
        assert!(!is_valid_domain(""));
        assert!(!is_valid_domain("-invalid.com"));
        assert!(!is_valid_domain("invalid-.com"));
        assert!(!is_valid_domain("invalid..com"));
        assert!(!is_valid_domain("invalid_domain.com"));
        assert!(!is_valid_domain("exam ple.com"));
    }

    #[test]
    fn test_validate_hosts_content_invalid_domain() {
        let content = "127.0.0.1 -invalid.com";
        assert!(validate_hosts_content(content, "https://example.com").is_err());
    }
}
