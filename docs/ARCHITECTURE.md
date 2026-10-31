# Mac电源助手架构

## 运行结构

- `crates/tpower`：IOKit、AppleSMC 与 MobileDevice 数据采集和归一化。
- `src-tauri`：Tauri 生命周期、设备连接状态机、SQLite 历史记录、菜单栏与 IPC。
- `src`：Vue 3 主窗口、设置窗口和菜单栏摘要。
- `locales`：中英文界面文案。
- `.sqlx`：SQLx 离线查询元数据，属于构建输入。

## 数据流

本机数据优先合并 IORegistry 与 AppleSMC；AppleSMC 不可用时继续使用 IORegistry。iOS 设备同时保留 USB 和 Wi‑Fi 连接，轮询优先使用 USB，断开后自动切换到 Wi‑Fi。

采样事件通过有序的无界通道进入历史状态机，避免异步任务重排。常规会话保留完整曲线；超过 24 小时后只压缩内部采样点，始终保留会话首尾。

## 安全边界

- WebView 启用 CSP。
- 前端没有全局文件写权限。
- 历史导出由受限 Rust 命令执行，只能写出数据库中的指定历史记录。
- FFI 连接失败必须回滚已建立的连接和 session。
