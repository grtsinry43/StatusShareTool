# onlineStatus API contract

来源：`/home/grtsinry43/grtblog-v2/server`

## 路由

- `GET /api/v2/onlineStatus`
- `POST /api/v2/onlineStatus`

## 鉴权

- `GET` 无需鉴权。
- `POST` 需要管理员权限。
- 后端实现兼容 `Bearer <jwt>` 和 `gt_...` 管理员 token。
- 本项目客户端需要传 `gt_...` 管理员 token，并直接把它放进 `Authorization` header。

## POST 请求体

```json
{
  "ok": 1,
  "process": "Coding",
  "extend": "Editing article",
  "category": "game",
  "game": {
    "name": "Stardew Valley",
    "cover": "https://example.com/game-banner.jpg",
    "slogan": "代码写不动了，就回鹈鹕镇种地。",
    "desc": "一句话描述",
    "accent": "#5da84f",
    "url": "https://store.steampowered.com/app/413150/"
  },
  "media": {
    "title": "Track Name",
    "artist": "Artist",
    "thumbnail": "https://example.com/cover.jpg",
    "position": 73.5,
    "duration": 214.0,
    "state": "playing"
  },
  "timestamp": 1742112000
}
```

字段说明：

- `ok`: 可选，仅允许 `0` 或 `1`
- `process`: 可选
- `extend`: 可选
- `category`: 可选，活动分类（如 `game`），小写字母/数字/`-`/`_`，最长 32；前端据此渲染特殊卡片
- `game`: 可选，游戏卡片元数据，随上报端规则维护（`category = game` 时展示）
  - `cover`/`url`：仅放行 `http(s)://` 链接，其余值会被服务端丢弃
  - 长度上限：`cover`/`url` 512、`slogan` 140、`desc` 280、`accent` 32
  - 全部子字段为空时视为未配置（上报端会直接省略该字段）
- `media`: 可选
  - `position`/`duration`：可选，单位秒；`position <= duration` 会被服务端钳制
  - `state`：可选，仅 `playing` / `paused` / `stopped` 会被保留，其余值会被丢弃
- `timestamp`: 可选，必须大于 `0`

调度约定：

- 上报端按心跳间隔（>= 5s）推送；`media.position` 与 `timestamp` 不参与“内容变化”指纹，
  避免进度前进导致每秒触发 Changed 推送，进度随心跳推送同步给前端。

## 成功响应 envelope

```json
{
  "code": 0,
  "bizErr": "OK",
  "msg": "success",
  "data": {
    "ok": 1,
    "process": "Coding",
    "extend": "Editing article",
    "category": "game",
    "game": {
      "cover": "https://example.com/game-banner.jpg",
      "slogan": "代码写不动了，就回鹈鹕镇种地。",
      "desc": "一句话描述",
      "accent": "#5da84f",
      "url": "https://store.steampowered.com/app/413150/"
    },
    "media": {
      "title": "Track Name",
      "artist": "Artist",
      "thumbnail": "https://example.com/cover.jpg",
      "position": 73.5,
      "duration": 214.0,
      "state": "playing"
    },
    "timestamp": 1742112000,
    "adminPanelOnline": false
  },
  "meta": {
    "requestId": "optional-request-id",
    "timestamp": "2026-03-16T00:00:00Z"
  }
}
```

## 服务端时效规则

- owner status 超过 5 分钟不更新会自动重置为离线。
- admin panel 心跳超过 90 秒会变成离线。
