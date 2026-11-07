# 验证记录

## v0.2.0 本地检查（2026-11-07）

- `cargo fmt --check`、`cargo clippy --locked --all-targets -- -D warnings`：通过，无警告。
- `cargo test --locked`：25 项通过，包括 17 项转换测试、4 项并发测试、4 项 UI / 对比度测试。
- 页序、0001 封面兼首页、XML / 导航、不同尺寸图片的视口、直接图片排版、零分页边距、原图不增加白边、取消清理和同名不覆盖均有覆盖。
- 并发测试验证实际重叠工作、并发上限、同名输出竞争、单本失败后继续、预先取消不启动任务。
- GUI 测试操作真实 AccessKit 控件，验证格式选择、主题、移除队列和键盘制作；非固定截图坐标测试。
- Linux 实际窗口截图检查 1440×900、1280×800、1024×768、800×600，以及深色、Rosé Pine 和选中队列状态。
- Windows x64 Release 实际启动；通过 Windows UI Automation 选中 PDF、开始制作 RAR、检查新生成的 PDF、切换主题并正常退出，均通过。
- Linux x86_64 Release GUI 实际在当前 WSL 桌面会话中启动、截图并正常退出；Release CLI 版本命令通过。
- Windows GNU 和 Linux 的 GUI / CLI Release 构建均通过。
- 新分页样式的 3 页 EPUB（纵向、横向、方形图片）通过 EPUBCheck 5.4.0：0 errors、0 warnings。

本地 Windows 二进制使用 `x86_64-pc-windows-gnu` 交叉构建。
GitHub Actions 的 Windows 作业使用本机 MSVC；Linux 使用 Ubuntu 22.04；macOS 作业生成 Intel + Apple Silicon universal2。
CI 的实际状态与对应提交产物以 [Actions](https://github.com/weiserTelysta/manga-to-epub/actions/workflows/build.yml) 为准。
本地没有 macOS 主机，不能把打包脚本就绪报告为 Mac 实机测试完成。

## 实体阅读器验证边界

用户报告汉王 N10 自带阅读器直接打开原始 EPUB 时每页右侧、底部留白；Kindle 在经 Calibre 转换后通过 USB 导入的文件上也有此现象。
仅补充分页零边距的 SVG 对照版在 N10 上仍缩在右上角；改为 XHTML 直接图片排版后，**用户实测恢复正常大小**。
正式输出采用相同的直接图片结构，保持视口和图片尺寸一致、原图字节不变；保留零分页边距与零行高，移除零字号。
**Kindle 尚未实机复测。** EPUBCheck 只验证格式，不能代替设备显示验证。
Calibre 9.18.0 的 EPUB → AZW3 对照显示：默认转换会将上下分页边距改为 5pt，显式零边距参数可以将其改为零；
中间 OPF 不再包含 EPUB 3 的固定版式标记。该结果只说明转换行为，不代表 Kindle 的最终显示已验证。
两条导入路径与对照方法分别记录在 [阅读器兼容性](READER-COMPATIBILITY.md)，不能将 N10 的问题归因于 Calibre。

## 首版历史验证

首版曾使用本地 226 页 WebP 图片包完成约 184.6 MiB EPUB，并通过 EPUBCheck 5.4.0：0 errors、0 warnings；原始输出见 `epubcheck.txt`。
原始漫画素材和完整电子书没有放入仓库。首版 Windows GUI / CLI 的中文路径 RAR → PDF 运行通过。
Windows 导入表检查未发现需随包附带的 UnRAR、libstdc++、winpthread 等第三方 DLL。
