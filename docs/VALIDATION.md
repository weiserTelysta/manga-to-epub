# 验证记录

- `cargo fmt --check`：通过。
- `cargo clippy --locked --all-targets -- -D warnings`：通过，无警告。
- `cargo test --locked`：15 项集成测试通过。
- GUI 实际启动并截图检查布局；通过界面导入 RAR、切换为 PDF、点击制作，成功生成 PDF。
- Windows x64 Release GUI 实际在 Windows 中启动，中文、Rosé Pine 配色与高 DPI 显示正常，截图见 `windows-preview.png`。
- Windows Release CLI 的版本命令运行成功；中文路径 RAR 成功转换为含中文标题的 PDF。
- 使用工作目录中的 `1487614.zip`，将 226 张 WebP 转换为 `Telysta Books/1487614.epub`（约 184.6 MiB）。
- 使用 [EPUBCheck 5.4.0](https://github.com/w3c/epubcheck/releases/tag/v5.4.0) 校验完整的 226 页 EPUB：0 errors、0 warnings，原始输出见 `epubcheck.txt`。
- 检查 Windows 可执行文件导入表：依赖均为 Windows 系统 DLL，无需附带 UnRAR、libstdc++、winpthread 等第三方 DLL。

当前交付的 Windows 二进制通过 Rust 的 `x86_64-pc-windows-gnu` 目标交叉构建；
`scripts/build-windows.ps1` 和 GitHub Actions 提供 Windows 本机的 MSVC 重新构建方式。
Windows GUI 的文件选择对话框使用 rfd 原生实现；本轮自动验证主要覆盖命令行导入与窗口中的制作流程。
