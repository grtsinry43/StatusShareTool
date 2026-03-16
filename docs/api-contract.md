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
  "media": {
    "title": "Track Name",
    "artist": "Artist",
    "thumbnail": "https://example.com/cover.jpg"
  },
  "timestamp": 1742112000
}
```

字段说明：

- `ok`: 可选，仅允许 `0` 或 `1`
- `process`: 可选
- `extend`: 可选
- `media`: 可选
- `timestamp`: 可选，必须大于 `0`

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
    "media": {
      "title": "Track Name",
      "artist": "Artist",
      "thumbnail": "https://example.com/cover.jpg"
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
