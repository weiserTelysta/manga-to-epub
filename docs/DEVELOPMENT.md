# 开发、测试与 Git

## 结构

| 文件 | 职责 |
| --- | --- |
| `src/input.rs` | 文件发现、自然排序、安全临时解压、RAR 互斥保护 |
| `src/book.rs` | 逐页处理、EPUB / PDF / CBZ 写入、原子保存 |
| `src/batch.rs` | 有界多线程工作池，批量错误隔离与取消 |
| `src/model.rs` | 输入、转换参数、进度与结果 |
| `src/ui.rs` | 桌面状态、事件与后台任务 |
| `src/ui/theme.rs` | 语义 Token 与主题对比度 |
| `src/ui/components.rs` | 按钮、图标、背景及导入区图形 |
| `src/ui/view.rs` | 桌面布局、设置、队列、键盘与浮层 |
| `src/ui/tests.rs` | 通过真实无障碍节点操作控件的 GUI 测试 |
| `src/bin/cli.rs` | 不依赖 GUI 的命令行入口 |

## 检查与打包

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo run --locked
```

`--no-default-features --bin telysta-cli` 可仅构建转换核心与 CLI。
Windows / Linux / macOS 打包脚本和运行依赖见 [平台说明](PLATFORMS.md)。

## 并发约束

`convert_batch` 为 GUI 和 CLI 复用的标准库线程池。并发数量有上限，回调携带稳定的任务索引。
不同书籍可同时进行；每本书内页序、封面选择与 ZIP 写入顺序不受完成先后影响。
输出通过临时文件和 `persist_noclobber` 原子提交，重复书名在并发下仍自动编号。
共享 UnRAR 状态只在解压阶段加锁，不将所有格式都串行化。

## UI 验证

UI 测试直接运行实际 egui 布局，通过 AccessKit 节点名称寻找控件并模拟点击与快捷键，避免依赖固定屏幕坐标。
截图检查涵盖 1440×900、1280×800、1024×768、800×600，另检查深色与任务状态。
Windows 原生 UI Automation 验证作为实际运行检查，源码测试可以在三个系统上执行。

诊断截图可使用 `TELYSTA_SCREENSHOT`、`TELYSTA_WINDOW_SIZE` 和 `TELYSTA_THEME` 环境变量；截图运行不保存窗口或主题偏好。

## Git 与发布

远端：`git@github.com:weiserTelysta/manga-to-epub.git`。
`v0.1.0` 保留首版可用实现；当前迭代在 `ui-redesign` 上开发，通过检查后合入 `main`。
原漫画素材、成品电子书、可执行产物和临时工具由 `.gitignore` 排除；仅很小的测试夹具随源码提交。

推送会触发 `.github/workflows/build.yml`，在 Windows、Ubuntu 与 macOS runner 上分别检查、构建并上传平台产物。
打标签前同步 Cargo、Windows manifest 和 macOS Info.plist 的版本。不要在缺少相应平台验证时将其报告为已验证。
