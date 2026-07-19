# Swift 绑定

当前提供供 macOS MVP 使用的最小 C ABI bridge，位于 `native`。它将 JSON 命令映射到工作区 Rust SDK，只公开两个内存边界清晰的函数：执行命令和释放结果。

暂未引入 UniFFI。等 iOS 应用或公开 Swift Package 确实需要类型化 API 时，再评估统一生成绑定。
