# StatusShareTool

跨平台桌面状态上报工具，面向 `grtblog-v2` 后端的 `/api/v2/onlineStatus` 接口。

核心流程是读取当前活动窗口，按匹配规则决定：

- 是否上报
- 上报什么外显名称
- 上报什么 `extend`
- 媒体信息是对象还是 `null`

## 技术栈

- Core: Rust + UniFFI
- Windows shell: WPF + Wpf.Ui
- Windows native bridge: Rust `cdylib` + P/Invoke
- macOS shell: SwiftUI
- Linux shell: `gtk-rs`

## 当前仓库状态

- 已实现 Rust core API 客户端和心跳逻辑
- 已实现窗口匹配规则的数据模型和决策逻辑
- 已实现配置 JSON 持久化与默认配置路径
- 已实现 Windows P/Invoke JSON bridge
- 已实现 Linux GTK 参考客户端
- 已写好 WPF / SwiftUI 工程骨架与接线代码
- 已补后端 API 契约与架构文档

## 快速开始

```bash
cargo check -p statusshare-core
cargo check -p windows-pinvoke
cargo run -p linux-gtk
```

## 目录

- `crates/statusshare-core`: 业务核心
- `crates/windows-pinvoke`: Windows 原生桥
- `apps/linux-gtk`: Linux 桌面端
- `apps/windows-wpf`: Windows 壳层
- `apps/macos-swiftui`: macOS 壳层
- `scripts/generate-swift-bindings.sh`: 生成 Swift UniFFI bindings
- `scripts/build-c-api.sh`: 构建 C API 动态库
- `docs/api-contract.md`: 后端接口说明
- `docs/architecture.md`: 项目结构与绑定策略

## Token 约束

这个项目的客户端需要传 `gt_` 开头的管理员 token。

## 配置持久化

推荐持久化方式是默认配置目录下的单文件 JSON：

- Windows: `%APPDATA%\\StatusShareTool\\config.json`
- macOS: `~/Library/Application Support/StatusShareTool/config.json`
- Linux: `$XDG_CONFIG_HOME/StatusShareTool/config.json`
  - 或 `~/.config/StatusShareTool/config.json`
