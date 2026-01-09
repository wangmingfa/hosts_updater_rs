//! 配置模块
//!
//! 提供配置文件的加载、解析和管理功能。

use anyhow::{Context, Result};
use serde::Deserialize;

/// 配置结构体
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// 更新间隔时间（小时）
    #[serde(default = "default_interval")]
    pub update_interval_hours: u64,
    /// hosts 数据源 URL 列表
    pub hosts_sources: Vec<String>,
    /// 更新前是否备份现有 hosts
    #[serde(default = "default_backup")]
    pub backup_before_update: bool,
    /// 备份文件保存路径
    #[serde(default)]
    pub backup_path: Option<String>,
}

fn default_interval() -> u64 {
    2
}

fn default_backup() -> bool {
    true
}

/// 加载配置
///
/// 按优先级顺序查找配置文件：
/// 1. 当前目录 (config.json/toml/yaml)
/// 2. 用户配置目录 (~/.config/hosts_updater/)
/// 3. 系统配置目录 (/etc/hosts_updater/)
pub fn load_config() -> Result<Config> {
    // 1. 尝试当前目录
    if let Some(config) = try_load_config("./config")? {
        return Ok(config);
    }

    // 2. 尝试用户配置目录
    if let Some(dir) = directories::UserDirs::new() {
        let user_config_path = dir.home_dir().join(".config/hosts_updater/config");
        if let Some(config) = try_load_config(&user_config_path.to_string_lossy())? {
            return Ok(config);
        }
    }

    // 3. 尝试系统配置目录
    if let Some(config) = try_load_config("/etc/hosts_updater/config")? {
        return Ok(config);
    }

    Err(anyhow::anyhow!("未找到配置文件"))
}

/// 尝试加载指定路径的配置
fn try_load_config(path: &str) -> Result<Option<Config>> {
    // 尝试 JSON 格式
    if let Ok(config) = load_json_config(&format!("{}.json", path)) {
        return Ok(Some(config));
    }

    // 尝试 TOML 格式
    if let Ok(config) = load_toml_config(&format!("{}.toml", path)) {
        return Ok(Some(config));
    }

    // 尝试 YAML 格式
    if let Ok(config) = load_yaml_config(&format!("{}.yaml", path)) {
        return Ok(Some(config));
    }

    Ok(None)
}

/// 加载 JSON 格式配置
fn load_json_config(path: &str) -> Result<Config> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("读取配置文件失败: {}", path))?;
    serde_json::from_str(&content).with_context(|| format!("解析 JSON 配置失败: {}", path))
}

/// 加载 TOML 格式配置
fn load_toml_config(path: &str) -> Result<Config> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("读取配置文件失败: {}", path))?;
    toml::from_str(&content).with_context(|| format!("解析 TOML 配置失败: {}", path))
}

/// 加载 YAML 格式配置
fn load_yaml_config(path: &str) -> Result<Config> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("读取配置文件失败: {}", path))?;
    let docs = yaml_rust::YamlLoader::load_from_str(&content)
        .with_context(|| format!("解析 YAML 配置失败: {}", path))?;
    let doc = &docs[0];

    // 将 yaml_rust::Yaml 转换为 serde_yaml::Value
    let value = convert_yaml_to_value(doc);
    serde_yaml::from_value(value).with_context(|| format!("转换 YAML 配置失败: {}", path))
}

/// 将 yaml_rust::Yaml 转换为 serde_yaml::Value
fn convert_yaml_to_value(yaml: &yaml_rust::Yaml) -> serde_yaml::Value {
    match yaml {
        yaml_rust::Yaml::Null => serde_yaml::Value::Null,
        yaml_rust::Yaml::Boolean(b) => serde_yaml::Value::Bool(*b),
        yaml_rust::Yaml::Integer(i) => serde_yaml::Value::Number((*i).into()),
        yaml_rust::Yaml::Real(s) => {
            if let Ok(num) = s.parse::<f64>() {
                serde_yaml::Value::Number(num.into())
            } else {
                serde_yaml::Value::String(s.clone())
            }
        }
        yaml_rust::Yaml::String(s) => serde_yaml::Value::String(s.clone()),
        yaml_rust::Yaml::Array(arr) => {
            serde_yaml::Value::Sequence(arr.iter().map(convert_yaml_to_value).collect())
        }
        yaml_rust::Yaml::Hash(map) => {
            let mut value_map = serde_yaml::Mapping::new();
            for (k, v) in map.iter() {
                let key = convert_yaml_to_value(k);
                let val = convert_yaml_to_value(v);
                value_map.insert(key, val);
            }
            serde_yaml::Value::Mapping(value_map)
        }
        _ => serde_yaml::Value::Null,
    }
}

/// 检查配置是否有效
pub fn validate_config(config: &Config) -> Result<()> {
    if config.hosts_sources.is_empty() {
        return Err(anyhow::anyhow!("hosts_sources 不能为空"));
    }

    for url in &config.hosts_sources {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow::anyhow!("无效的 URL: {}", url));
        }
    }

    Ok(())
}
