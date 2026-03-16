# AGENTS.md

## 项目目标

这个仓库实现一个跨平台桌面状态上报工具，对接 `grtblog-v2` 的 `/api/v2/onlineStatus`。

## 目录约定

- `crates/statusshare-core`
  - 唯一业务核心来源。
  - 任何接口字段、鉴权策略、心跳规则变更，先改这里。
- `crates/windows-pinvoke`
  - 仅做稳定 ABI 桥接，不重复业务逻辑。
  - 对外保持 JSON in / JSON out。
- `apps/linux-gtk`
  - Linux 可运行参考实现。
- `apps/windows-wpf`
  - WPF + Wpf.Ui 壳层。
  - 只做视图、定时器、配置收集、调用 native bridge。
- `apps/macos-swiftui`
  - SwiftUI 壳层。
  - 依赖 UniFFI Swift binding。
- `docs`
  - 后端契约、架构、构建说明。

## 开发规则

- 改 API 前先核对后端：
  - `/home/grtsinry43/grtblog-v2/server/internal/http/handler/owner_status_handler.go`
  - `/home/grtsinry43/grtblog-v2/server/internal/app/ownerstatus/service.go`
  - `/home/grtsinry43/grtblog-v2/server/internal/http/middleware/auth_middleware.go`
- `statusshare-core` 必须保持平台无关，不引入 GUI 依赖。
- Windows 壳层不得直接重写 HTTP 逻辑，必须经 `windows-pinvoke` 或 core。
- macOS 壳层不得自己拼接口字段，必须以 UniFFI 导出的类型为准。
- 如果只改 UI，不要修改 core 的请求/响应模型命名。
- 窗口匹配规则、外显名称、extend 文案、白名单黑名单判定统一放在 `statusshare-core`。
- 配置读写统一走 core 的 JSON 持久化接口，不要各平台各自定义配置格式。

## API 提取结果

以下内容直接从后端代码提取，变更前必须先重新核对后端实现。

### 路由

- `GET /api/v2/onlineStatus`
  - 注册位置：`internal/http/router/public_routes.go`
  - 作用：公开获取当前站长在线状态
- `POST /api/v2/onlineStatus`
  - 注册位置：`internal/http/router/admin_routes.go`
  - 作用：管理员上报最新在线状态
- `POST /api/v2/admin/owner-status/panel-heartbeat`
  - 注册位置：`internal/http/router/admin_routes.go`
  - 作用：后台管理面板心跳，和桌面状态上报不是同一接口

### 鉴权

- `GET /api/v2/onlineStatus`：不需要鉴权
- `POST /api/v2/onlineStatus`：必须通过 `RequireAuth + RequireAdmin`
- 后端实现上接受两类 token：
  - `Authorization: Bearer <jwt>`
  - `Authorization: gt_xxx`
- 后端 `extractToken()` 逻辑说明：
  - 如果 header 整体以 `gt_` 开头，直接当管理员 token
  - 否则要求 `Bearer <token>`
- 本项目客户端传参约定：
  - `token` 字段填写 `gt_` 开头的管理员 token
  - 请求发送时，直接把该值原样写入 `Authorization`

### POST 请求体规范

来源：`internal/http/handler/owner_status_handler.go` 和 `internal/app/ownerstatus/service.go`

请求体允许为空；空体时服务端会把 `ok` 归一化成 `1`。

```json
{
  "ok": 1,
  "process": "Coding",
  "extend": "Writing desktop client",
  "media": {
    "title": "Song",
    "artist": "Artist",
    "thumbnail": "https://example.com/image.jpg"
  },
  "timestamp": 1742112000
}
```

字段语义：

- `ok`
  - 类型：`int`
  - 可选
  - 只允许 `0` 或 `1`
  - 省略时后端会归一化为 `1`
- `process`
  - 类型：`string`
  - 可选
  - 后端会 `TrimSpace`
- `extend`
  - 类型：`string`
  - 可选
  - 后端会 `TrimSpace`
- `media`
  - 类型：对象，可选
  - 子字段：
    - `title: string`
    - `artist: string`
    - `thumbnail: string`
  - 三个字段全空时，后端会把整个 `media` 视为 `nil`
- `timestamp`
  - 类型：`int64`
  - 可选
  - 必须大于 `0`
  - 省略时后端使用当前 Unix 时间戳

### 响应 envelope

来源：`internal/http/response/response.go`

所有接口都走统一 envelope：

```json
{
  "code": 0,
  "bizErr": "OK",
  "msg": "success",
  "data": {},
  "meta": {
    "requestId": "",
    "timestamp": "2026-03-16T00:00:00Z"
  }
}
```

`/api/v2/onlineStatus` 的 `data` 结构：

```json
{
  "ok": 1,
  "process": "Coding",
  "extend": "Writing desktop client",
  "media": {
    "title": "Song",
    "artist": "Artist",
    "thumbnail": "https://example.com/image.jpg"
  },
  "timestamp": 1742112000,
  "adminPanelOnline": false
}
```

字段语义：

- `ok: int`
- `process: string`
- `extend: string`
- `media: object | null`
- `timestamp: int64`
- `adminPanelOnline: bool`

### 服务端时效规则

来源：`internal/app/ownerstatus/service.go`

- owner status 超过 `5 分钟`未更新，服务端会自动重置为离线：
  - `ok = 0`
  - `process = ""`
  - `extend = ""`
  - `media = nil`
- admin panel heartbeat 超过 `90 秒`未刷新，`adminPanelOnline = false`
- 服务端后台每 `10 秒`做一次过期检查

### 实现要求

- 桌面客户端只对接 `/api/v2/onlineStatus`
- 不要把 panel heartbeat 混进桌面状态上报流程
- core 层必须统一做：
  - URL 规范化
  - `gt_` 管理员 token 读取与 header 写入
  - envelope 解析
  - `adminPanelOnline` 响应字段映射，只读处理
  - 心跳间隔控制

## 窗口匹配机制

这个项目的核心流程是：

1. 平台层获取当前活动窗口
2. 平台层获取当前媒体信息
3. 把这些信息交给 core
4. core 根据规则决定是否上报，以及上报什么 `process` / `extend`

### 平台层输入

每个平台都要尽量提供这些字段给 core：

- `window_title`
- `app_name`
- `process_name`
- `executable_path`
- `bundle_id`
- `media`
  - 有媒体信息就传对象
  - 没有就传 `null`

### 规则结构

每条窗口规则包含：

- `field`
  - `window_title`
  - `app_name`
  - `process_name`
  - `executable_path`
  - `bundle_id`
- `kind`
  - `contains`
  - `exact`
  - `prefix`
  - `suffix`
- `pattern`
- `report_policy`
  - `allow`
  - `deny`
- `display_name`
  - 命中后上报到后端 `process` 的外显名称
- `extend`
  - 命中后上报到后端 `extend` 的描述

### 行为语义

- `allow`
  - 命中后执行上报
  - `process` 使用规则里的 `display_name`
  - `extend` 使用规则里的 `extend`
- `deny`
  - 命中后本次不上报

### 默认行为

全局需要一个默认策略：

- `default_report = true`
  - 未命中规则时也上报
- `default_report = false`
  - 未命中规则时不上报

未命中但允许上报时：

- `process` 优先取全局 `default_display_name`
- 没配时回退到 `app_name` / `process_name` / `window_title`
- `extend` 取全局 `default_extend`

### 字段映射约定

- 后端 `process`
  - 表示窗口外显名称
  - 例如把 `kitty` 外显为 `Kitty`
- 后端 `extend`
  - 表示这条窗口对应的一句话
  - 例如 `Kitty -> 没准正在 yay -Syyu，希望不要 grub>`
- 后端 `media`
  - 有值就上报对象
  - 没值就上报 `null`

### 实现边界

- 平台层负责：
  - 获取当前活动窗口
  - 获取媒体信息
  - 提供规则编辑界面
- core 负责：
  - 匹配规则判断
  - 白名单黑名单判定
  - `process` / `extend` / `media` 组装
  - 产出最终 `StatusUpdate`

## 配置持久化

配置推荐使用默认配置目录里的 JSON 单文件，由 core 负责：

- Windows
  - `%APPDATA%\\StatusShareTool\\config.json`
- macOS
  - `~/Library/Application Support/StatusShareTool/config.json`
- Linux
  - `$XDG_CONFIG_HOME/StatusShareTool/config.json`
  - 回退到 `~/.config/StatusShareTool/config.json`

core 对外暴露：

- `default_config_file_path`
- `default_persisted_config`
- `load_persisted_config`
- `save_persisted_config`

持久化的内容至少包含：

- 上报基础配置
  - `base_url`
  - `token`
  - `heartbeat_interval_secs`
  - `user_agent`
- 窗口匹配配置
  - 默认行为
  - 默认外显名
  - 默认 extend
  - 规则列表

## 对外绑定

- UniFFI
  - core 通过 UniFFI 导出给 Swift
  - 生成脚本：`./scripts/generate-swift-bindings.sh`
- C API
  - `crates/windows-pinvoke/include/statusshare_c_api.h`
  - 构建脚本：`./scripts/build-c-api.sh`
  - 主要导出：
    - `ss_fetch_status`
    - `ss_push_status`
    - `ss_default_config_file_path`
    - `ss_default_persisted_config`
    - `ss_load_persisted_config`
    - `ss_save_persisted_config`
    - `ss_resolve_status_update`
    - `ss_string_free`

## 推荐命令

```bash
cargo check -p statusshare-core
cargo check -p windows-pinvoke
cargo run -p linux-gtk
```

## 当前已知限制

- WPF 工程需要在 Windows 上用 .NET SDK 构建。
- SwiftUI 工程需要在 macOS 上生成并引入 UniFFI Swift bindings。
- Linux GTK 需要系统安装 GTK4 开发依赖。
