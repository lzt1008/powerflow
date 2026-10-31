# 发布流程

当前应用名为“Mac电源助手”，首个版本为 `0.1.0`，作者为 `cnveteran`。

1. 完成代码审查和本地真机试用。
2. 运行 `pnpm lint`、`pnpm typecheck`、`pnpm test`、`cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace` 和 `pnpm build`。
3. 更新版本元数据并提交。
4. 创建 annotated tag，例如 `git tag -a v0.1.0 -m "Mac电源助手 0.1.0"`。
5. 推送 tag 后，由 GitHub Actions 构建 universal macOS 安装包并创建草稿 Release。
6. 人工验证安装包后再公开 Release。

未经本地试用确认，不创建 tag、不推送 Release。源码历史仅通过 Git commit/tag 保存，本地安装包放在被忽略的 `artifacts/releases/`。
