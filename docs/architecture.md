# Architecture

## Workspace

```text
StatusShareTool/
├── crates/
│   ├── statusshare-core/
│   └── windows-pinvoke/
├── apps/
│   ├── linux-gtk/
│   ├── windows-wpf/
│   └── macos-swiftui/
└── docs/
```

## Rust core

`crates/statusshare-core` 是整个项目唯一的业务实现源：

- 负责 `/api/v2/onlineStatus` 的请求拼装和响应解析
- 负责 token header 规范化
- 负责心跳线程
- 负责窗口匹配规则和上报决策
- 负责配置持久化
- 通过 `uniffi` 暴露给 Swift
- 直接给 `gtk-rs` 使用
- 被 `windows-pinvoke` 复用，提供稳定 C ABI 给 WPF

## Config Persistence

配置统一落在单文件 JSON 中，由 core 负责读写：

- 原因
  - 规则结构是嵌套数组和对象，JSON 序列化最直接
  - C API、WPF、SwiftUI、gtk-rs 都容易处理 JSON
  - 中文 `extend` 文案和规则数据不需要额外转义策略
- 默认路径
  - Windows: `%APPDATA%\\StatusShareTool\\config.json`
  - macOS: `~/Library/Application Support/StatusShareTool/config.json`
  - Linux: `$XDG_CONFIG_HOME/StatusShareTool/config.json`，否则 `~/.config/StatusShareTool/config.json`
- 对外接口
  - `default_config_file_path()`
  - `default_persisted_config()`
  - `load_persisted_config(path)`
  - `save_persisted_config(path, config)`

## Matching

窗口匹配逻辑统一放在 Rust core，而不是散落在三端 UI：

- 输入
  - 当前活动窗口：
    - `window_title`
    - `app_name`
    - `process_name`
    - `executable_path`
    - `bundle_id`
  - 当前媒体信息：
    - 有媒体就传对象
    - 没有就传 `null`
- 规则
  - 匹配字段
  - 匹配方式
  - 白名单或黑名单
  - 外显名称
  - `extend` 描述
- 输出
  - 是否上报
  - 命中的规则 ID
  - 最终 `StatusUpdate`

这样平台层只负责采集系统信息和管理配置，core 负责最终决策。

## Windows

Windows 不直接消费 UniFFI，而是走独立桥接库：

- `crates/windows-pinvoke`: `cdylib`
- 导出：
  - `ss_fetch_status`
  - `ss_push_status`
  - `ss_default_config_file_path`
  - `ss_default_persisted_config`
  - `ss_load_persisted_config`
  - `ss_save_persisted_config`
  - `ss_resolve_status_update`
  - `ss_string_free`

WPF 通过 `DllImport` 调用，数据格式统一为 JSON 字符串，降低 ABI 演进成本。

## macOS

macOS 壳层用 SwiftUI，目标是消费 UniFFI 生成的 Swift binding。

推荐生成流程：

1. 构建 Rust core 静态库或动态库。
2. 用 `uniffi-bindgen` 生成 Swift binding。
3. 把生成文件引入 `apps/macos-swiftui` 的 Xcode 工程。

示例命令：

```bash
cargo build -p statusshare-core --release
cargo run --bin uniffi-bindgen generate \
  --library target/release/libstatusshare_core.a \
  --language swift \
  --out-dir apps/macos-swiftui/Generated
```

具体产物名会随目标平台变化，macOS 上通常是 `.a` 或 `.dylib`。

仓库提供脚本：

```bash
./scripts/generate-swift-bindings.sh
```

## Linux

Linux 首版使用 `gtk-rs`，直接依赖 `statusshare-core`，当前仓库中它是参考实现和联调工具。
