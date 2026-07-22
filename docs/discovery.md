# 设备发现

OpenEdifier 浏览 `_airplay._tcp.local.`，因为已验证的 S260 会通过该服务公布身份，但不会广播单独的控制服务。

当 TXT 中的 `manufacturer` 或服务名称能够识别 EDIFIER 时，该广播会被视为 EDIFIER 候选设备。S260 广播提供：

- 稳定的设备 ID
- 便于阅读的实例名称
- `.local` 主机名和 IP 地址
- EDIFIER 型号代码

广播的 AirPlay 端口不用于控制。只有经过验证的 AirPlay 硬件型号 `EDF100122` 会映射到 S260 驱动，即使用户重命名了服务也不受影响；实例名称不能决定驱动。驱动随后选择经过验证的 TCP 控制端口 `8080`。

低层 `open-edifier-discovery::discover_candidates` 会保留未知 EDIFIER 型号，用于研究和增加新驱动；高层 `open_edifier::discover` 只返回当前 build 已支持的设备，CLI、C ABI 和应用统一使用高层语义。

发现过程会完整等待调用方指定的时长，避免较慢的设备在首个响应后被遗漏。存在多台受支持设备时，调用方必须明确选择一台。
