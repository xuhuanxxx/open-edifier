# S260 局域网协议

本文描述在固件 `01.00.00` 的 EDIFIER S260 上观察到的行为。

## 传输

- TCP 端口：`8080`
- 当前 S260 传输加密：无
- 请求：紧凑 UTF-8 JSON，无帧头或分隔符
- 响应：`EE DD FF EE`，随后是两字节大端载荷长度和 UTF-8 JSON
- socket 中也可能出现 `BB EC 3F ...` 模组心跳。decoder 必须扫描响应头，不能假设帧按读取边界对齐。

## 请求

状态：

```json
{"id":"<unique-id>","payload":"status_query"}
```

输入源：

```json
{"id":"<unique-id>","payload":"settings","inputSource":{"inputIndex":1,"selectedIndex":2}}
```

输入源编号：

| 值 | 输入源 |
|---:|---|
| 0 | 蓝牙 |
| 1 | AUX |
| 2 | USB |
| 3 | AirPlay |

音量：

```json
{"id":"<unique-id>","payload":"settings","player":{"volume":18}}
```

EQ 预设：

```json
{"id":"<unique-id>","payload":"settings","soundEffect":{"selectedIndex":1}}
```

已验证设备通过 `soundEffect.soundIndex` 报告三个预设。它还报告了六段 DIY EQ 结构，但 OpenEdifier 不开放对该未文档化结构的写入。

播放控制：

```json
{"id":"<unique-id>","payload":"settings","player":{"playerStatus":1}}
{"id":"<unique-id>","payload":"settings","player":{"playerStatus":0}}
{"id":"<unique-id>","payload":"settings","player":{"next":1}}
{"id":"<unique-id>","payload":"settings","player":{"previous":1}}
```

播放命令在蓝牙、AirPlay 等具备媒体能力的输入源下生效。在 USB 或 AUX 下，设备可能确认命令但不改变播放状态。

SDK 会在写入前读取设备报告的范围。输入源、音量和 EQ 修改会在有截止时间的验证窗口内重复查询状态，允许 ACK 与状态投影之间存在短暂延迟；到期仍不匹配时返回包含目标值、最后观察值、查询次数和耗时的结构化错误。

输入源验证描述的是命令被确认后紧接着观察到的状态。所选输入源不可用时，音箱之后仍可能回退到其他输入源；USB 线已断开时选择 USB 已复现该行为。需要持续状态的应用还应消费二进制事件流或主动刷新状态。

独立的实时事件通道见 [AA EC/BB EC 协议](aaec.md)。

## 安全边界

观察到的 S260 控制通道既没有认证，也没有加密。不要在可信局域网外暴露 TCP `8080` 端口。响应必须按不可信输入处理，并且必须包含匹配的请求 ID、整数结果代码和明确的成功消息。
