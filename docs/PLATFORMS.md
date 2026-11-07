# Windows / Linux / macOS

同一套 Rust 核心与 eframe GUI 支持三个系统。平台差异集中在原生窗口、文件选择框、快捷键和应用打包。

| 系统 | 产物 | 运行方式 |
| --- | --- | --- |
| Windows 10 / 11 x64 | `Telysta-Windows-x64.zip` | 解压后双击 `Telysta.exe` |
| Linux x86_64 | `Telysta-Linux-x86_64.tar.gz` | 解压后运行 `./telysta` |
| macOS 11+，Intel / Apple Silicon | `Telysta-macOS-universal.zip` | 解压后打开 `Telysta.app` |

三种包均提供 `telysta-cli`，Windows 下为 `telysta-cli.exe`。运行不需要 Rust、Python、Node.js、WebView 或 7-Zip。

## 下载与构建状态

发布版本从 [GitHub Releases](https://github.com/weiserTelysta/manga-to-epub/releases/latest) 下载。在 Assets 中选择对应系统的包，`SHA256SUMS.txt` 提供各包的校验值。

开发中的三平台构建在 [GitHub Actions](https://github.com/weiserTelysta/manga-to-epub/actions/workflows/build.yml) 中进行。打开成功的构建记录，从 Artifacts 下载对应系统的包。GitHub 可能要求登录后才能下载 Actions 产物，当前保存期为 90 天。

本机生成的产物位于 `dist/`。macOS 包必须在 Mac 或 GitHub 的 macOS runner 上生成，Linux 环境不伪造 Mac 构建结果。
具体完成与验证情况见 [验证记录](VALIDATION.md)。

## Windows

直接双击可执行文件，也可把本地压缩包拖到窗口。文件和文件夹选择框为 Windows 原生对话框。

当前 Windows 包尚未进行 Authenticode 代码签名，首次启动可能出现 SmartScreen 的“Windows 已保护你的电脑”和“发布者未知”。
从本项目 Release 下载后，可在 PowerShell 中核对压缩包的 SHA-256，与同一 Release 中的 `SHA256SUMS.txt` 比较：

```powershell
Get-FileHash .\Telysta-Windows-x64.zip -Algorithm SHA256
```

确认来源和校验值后，点击“更多信息 → 仍要运行”；如果界面已经显示“仍要运行”，可直接点击。
可信代码签名用于验证发布者身份，新签名程序仍可能在积累信誉期间收到提示，见 [Microsoft 的 SmartScreen 说明](https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation)。

源码构建需要 Rust 1.93+、Visual Studio C++ Build Tools 与 Windows SDK：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-windows.ps1
```

脚本执行检查、测试、Release 构建及 ZIP 打包。MSVC 使用静态 CRT；解压库内置，不依赖外部 UnRAR DLL。

## Linux

支持 X11 / Wayland，通过 OpenGL 显示界面。Linux 包使用系统桌面库；正常桌面环境通常已安装其中大部分。
Ubuntu / Debian 可安装：

```sh
sudo apt-get install libx11-6 libxcb1 libxkbcommon0 libxkbcommon-x11-0 libwayland-client0 libegl1 libgl1 libxcursor1 libxi6 libxrandr2 zenity
```

`zenity` 用作原生文件对话框后端；桌面门户服务也可以提供文件选择。
GitHub 的 x86_64 包在 Ubuntu 22.04 上构建，以降低 glibc 兼容性要求。其他发行版可以自行用源码构建。
在更新发行版上本地编译的二进制不保证能直接运行于旧发行版。

源码构建：

```sh
sudo apt-get install build-essential pkg-config libxkbcommon-dev libwayland-dev libgl1-mesa-dev libxcb-shape0-dev libxcb-xfixes0-dev
bash scripts/build-linux.sh
```

解压后运行 `./telysta`。包内提供 `.desktop` 启动器模板；如需添加到系统应用菜单，将可执行文件放入 PATH，并安装图标与桌面文件。

## macOS

包内的 `.app` 和 CLI 均为 universal2，包含 Intel x86_64 与 Apple Silicon arm64 两个架构。
快捷键使用 Command，例如 `⌘O`、`⌘⇧O`、`⌘Enter`。

源码构建需要 Rust 1.93+ 与 Xcode Command Line Tools：

```sh
xcode-select --install
bash scripts/build-macos.sh
```

脚本在 Mac 上构建两个 Rust target，通过 `lipo` 合并，生成 `.app`、检查架构并做 ad-hoc 签名，再打包 ZIP。
当前开发包没有 Apple Developer ID 公证；下载后是否需要在“系统设置 → 隐私与安全性”中允许打开，取决于系统的下载来源检查。

## 验证边界

自动化流程在每个平台运行格式检查、Clippy、转换测试、主题对比度和 GUI 交互测试。
Linux 流程额外在 Xvfb 中启动实际 GUI 并截图，macOS 流程验证 universal2 架构和签名。
真实屏幕阅读器、所有桌面环境和所有旧版系统仍需要相应设备上的人工验证。
