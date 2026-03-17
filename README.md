# StatusShareTool

<p align="center">
  <img src="assets/icon.png" alt="StatusShareTool" width="128" />
</p>

<p align="center">
  <strong>grtblog-v2 站长状态上报工具</strong>
</p>

<p align="center">
  跨平台桌面客户端，实时将你的电脑使用状态同步到博客，让访客知道站长正在做什么。
</p>

---

StatusShareTool 是 [grtblog-v2](https://github.com/grtsinry43/grtblog) 生态中的桌面端组件。它在后台静默运行，自动检测当前活动窗口和正在播放的媒体信息，并按照你配置的匹配规则上报到博客后端，在站点首页展示站长的实时状态。

## 功能特性

- **活动窗口检测** — 自动识别当前正在使用的应用程序
- **媒体播放检测** — 获取正在播放的音乐/视频信息（标题、艺术家、封面等）
- **灵活的匹配规则** — 自定义哪些应用需要上报、显示什么名称、附带什么扩展信息
- **跨平台支持** — macOS / Windows / Linux 三端可用
- **后台静默运行** — 系统托盘常驻，不打扰日常使用

## 下载安装

前往 [Releases](https://github.com/grtsinry43/StatusShareTool/releases) 页面下载对应平台的安装包：

| 平台 | 格式 | 说明 |
|------|------|------|
| macOS (Intel & Apple Silicon) | `.dmg` | 拖入 Applications 即可使用 |
| Windows (x64) | `.exe` 安装包 | 运行安装向导 |
| Linux (x86_64) | `.AppImage` / `.deb` | AppImage 直接运行；deb 用 `dpkg -i` 安装 |

## 使用方式

1. 在 grtblog-v2 后台获取管理员 Token（`gt_` 开头）
2. 打开 StatusShareTool，填入后端地址和 Token
3. 配置窗口匹配规则（可选）
4. 保持后台运行，状态会自动同步到博客

## 技术架构

```
┌──────────────────────────────────────────────┐
│              statusshare-core (Rust)          │
│         业务逻辑 · 规则引擎 · API 客户端       │
└──────┬──────────────┬──────────────┬─────────┘
       │ UniFFI/Swift │ cdylib/P/Invoke│ 直接依赖
┌──────▼──────┐ ┌─────▼───────┐ ┌─────▼──────┐
│ macOS       │ │ Windows     │ │ Linux      │
│ SwiftUI     │ │ WPF+Wpf.Ui │ │ GTK4       │
└─────────────┘ └─────────────┘ └────────────┘
```

## 配置文件

配置以 JSON 格式存储在系统标准配置目录下：

| 平台 | 路径 |
|------|------|
| Windows | `%APPDATA%\StatusShareTool\config.json` |
| macOS | `~/Library/Application Support/StatusShareTool/config.json` |
| Linux | `~/.config/StatusShareTool/config.json` |

## 从源码构建

```bash
# 检查核心库
cargo check -p statusshare-core

# 运行 Linux 客户端
cargo run -p statussharetool

# Windows 客户端（需要 .NET 8 SDK）
cd apps/windows-wpf/StatusShare.Wpf && dotnet run

# macOS 客户端（需要 Xcode + xcodegen）
cd apps/macos-swiftui && xcodegen generate && open StatusShareMac.xcodeproj
```

## 项目结构

```
crates/statusshare-core    # Rust 业务核心
crates/windows-pinvoke     # Windows 原生桥接 (cdylib)
apps/macos-swiftui         # macOS 客户端
apps/windows-wpf           # Windows 客户端
apps/linux-gtk             # Linux 客户端
installer/                 # Windows Inno Setup 安装脚本
docs/                      # API 契约与架构文档
```

## License

MIT
