# OpenEdifier Python 客户端

`open-edifier` 是面向自动化和 Home Assistant 的纯 Python 异步客户端。它直接连接经过验证的 S260 协议，不调用 CLI，也不加载 Rust 动态库。

当前包提供：

- 主机直连和显式生命周期管理；
- 状态查询；
- 带写后验证的输入源、音量和 EQ 修改；
- 播放命令；
- 可取消、断线重连的实时事件流；
- 结构化错误和不包含厂商完整响应的公共状态。

设备发现不属于这个包。Home Assistant 集成应使用平台已有的 Zeroconf 发现，再将主机地址交给客户端，避免重复维护 mDNS 运行时。

## 本地安装

需要 Python 3.11 或更高版本：

```bash
python3 -m pip install ./bindings/python
```

## 使用

```python
import asyncio

from open_edifier import S260Client


async def main() -> None:
    async with S260Client("192.0.2.10") as client:
        status = await client.status()
        print(status.volume)
        await client.set_volume(18)


asyncio.run(main())
```

事件通道使用独立连接，并可通过取消任务停止：

```python
from open_edifier import S260EventStream


async with S260EventStream("192.0.2.10") as events:
    async for event in events:
        print(event)
```

S260 当前使用未经认证和加密的局域网协议。只能连接可信局域网中的设备，不要将控制端口暴露到互联网。

## 验证

```bash
cd bindings/python
PYTHONPATH=src python3 -m unittest discover -s tests
python3 -m pip wheel --no-deps --wheel-dir /tmp/open-edifier-wheel .
```
