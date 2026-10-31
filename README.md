# Mac电源助手

Mac电源助手是一款 macOS 菜单栏应用，用于实时查看 Mac 与已连接 iOS 设备的功耗、充放电状态、剩余时间和充电历史。

## 功能

- 实时功耗与电池状态监控
- Mac 与 iOS 设备切换
- 系统、屏幕、散热与电池功率分解
- 充电会话记录、趋势图和 JSON 导出
- 菜单栏快速状态与独立设置窗口
- 浅色、深色和系统主题

## 开发

```bash
pnpm install
pnpm tauri dev
```

构建前请运行：

```bash
pnpm lint
pnpm exec vue-tsc --noEmit
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
pnpm build
```

## 发布

正式版本只通过本仓库的 [GitHub Releases](https://github.com/cnveteran/powerflow/releases) 发布。版本历史以 Git tag 和 Release 为准，不在仓库中保存源码快照副本。

## 作者

cnveteran

## 致谢

本项目基于 [The Powerflow Team](https://github.com/lzt1008/powerflow) 的 powerflow 项目二次开发，感谢原作者的开源贡献。

## 许可证

[MIT](LICENSE)
