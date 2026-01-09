# hosts_updater_rs

一个用 Rust 编写的 Hosts 文件自动更新工具，定时从配置源获取 hosts 规则并写入系统 hosts 文件，帮助实现域名访问加速。

## 功能特性

- **定时更新**：每隔指定时间自动从配置源获取最新 hosts 规则
- **灵活配置**：支持自定义更新间隔，默认为 2 小时
- **权限检测**：自动检测管理员权限，必要时给出提示
- **跨平台支持**：基于 Rust 实现，天然具备跨平台能力

## 快速开始

### 环境要求

- Rust 1.70.0 或更高版本
- 管理员/root 权限（用于写入 hosts 文件）

### 编译运行

```bash
#  Debug 模式编译运行
cargo run --release

#  或直接运行编译后的二进制文件
./target/release/hosts_updater_rs
```

> ⚠️ 程序需要管理员权限才能修改系统 hosts 文件。

## 配置说明

程序通过配置文件指定 hosts 数据源和相关参数，支持 JSON/TOML/YAML 格式。

### 配置文件示例

**config.json**
```json
{
  "update_interval_hours": 2,
  "hosts_sources": [
    "https://example.com/hosts1",
    "https://example.com/hosts2"
  ],
  "backup_before_update": true,
  "backup_path": "./backup/hosts.backup"
}
```

**config.toml**
```toml
update_interval_hours = 2
backup_before_update = true
backup_path = "./backup/hosts.backup"

[hosts_sources]
urls = [
    "https://example.com/hosts1",
    "https://example.com/hosts2"
]
```

### 配置项说明

| 配置项 | 类型 | 必填 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `update_interval_hours` | Number | 否 | 2 | 更新间隔时间（小时） |
| `hosts_sources` | Array | 是 | - | hosts 数据源 URL 列表（返回内容必须为纯文本格式，可直接追加到系统 hosts 文件） |
| `backup_before_update` | Boolean | 否 | true | 更新前是否备份现有 hosts |
| `backup_path` | String | 否 | - | 备份文件保存路径 |

### 配置文件位置

程序会自动在以下位置查找配置文件（按优先级顺序）：

1. `./config.json` / `./config.toml` / `./config.yaml`（当前目录）
2. `~/.config/hosts_updater/config.json`（用户配置目录）
3. `/etc/hosts_updater/config.json`（系统配置目录）

### 数据源返回格式要求

`hosts_sources` 中每个 URL 返回的内容必须是纯文本格式，可直接追加到系统 hosts 文件。示例：

```
# 注释行（以 # 开头）
127.0.0.1 localhost
192.168.1.100 example.com
192.168.1.101 api.example.com
```

**格式要求：**
- 每行一条记录，格式为 `<IP> <域名>`
- 支持 `#` 开头的注释行
- 支持空行
- 不支持复杂的配置指令

### hosts 文件插入格式

程序会自动在系统 hosts 文件中插入一段带标记的内容，便于后续更新时精确替换。格式如下：

```
# >>> hosts_updater_rs START >>>
# 此区域由 hosts_updater_rs 自动管理，请勿手动修改
# 最后更新: 2024-01-15 10:30:00

127.0.0.1 localhost
192.168.1.100 example.com
192.168.1.101 api.example.com

# <<< hosts_updater_rs END <<<
```

**标记说明：**
- **开始标记**：`# >>> hosts_updater_rs START >>>`
- **结束标记**：`# <<< hosts_updater_rs END <<<`
- **更新逻辑**：程序每次更新时会先查找这两个标记之间的内容，将其删除后替换为新的 hosts 规则
- **手动处理**：如果标记缺失或损坏，程序会提示用户手动处理或追加到文件末尾

**多数据源示例：**

```
# >>> hosts_updater_rs START >>>
# 此区域由 hosts_updater_rs 自动管理，请勿手动修改
# 最后更新: 2024-01-15 10:30:00

# Source: https://example.com/hosts1
127.0.0.1 localhost
192.168.1.100 example.com

# Source: https://example.com/hosts2
192.168.1.101 api.example.com
192.168.1.102 docs.example.com

# <<< hosts_updater_rs END <<<
```

**说明：**
- 每个数据源的 hosts 内容前会添加 `# Source: <URL>` 注释标记
- 便于追溯各条记录的来源
- 更新时会按数据源顺序重新生成，保持结构清晰

## 项目结构

```
hosts_updater_rs/
├── src/
│   ├── main.rs       # 程序入口
│   ├── config.rs     # 配置模块：配置文件加载、解析和验证
│   ├── hosts.rs      # hosts 文件管理：读写、备份、标记处理
│   ├── fetcher.rs    # 网络获取模块：从 URL 获取 hosts 内容
│   └── scheduler.rs  # 定时任务模块：定时执行更新任务
├── Cargo.toml        # 项目配置
└── README.md         # 项目文档
```

### 模块说明

| 模块 | 职责 |
|------|------|
| `config.rs` | 负责加载和解析 JSON/TOML/YAML 格式的配置文件 |
| `hosts.rs` | 负责系统 hosts 文件的读写、备份和标记区域管理 |
| `fetcher.rs` | 负责从配置的 URL 获取 hosts 内容，支持 HTTP/HTTPS |
| `scheduler.rs` | 负责定时任务的调度，支持自定义更新间隔 |
| `main.rs` | 程序入口，协调各模块工作 |

## License

MIT License
