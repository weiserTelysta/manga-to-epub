# Telysta · 漫画成书

使用 Rust 编写的跨平台漫画图片电子书工具。面向 JM（禁漫天堂）导出的本地图片压缩包，也支持自己收集并按编号整理的图片。
默认生成 EPUB，可选 PDF / CBZ；排序后的第一张图片（推荐命名为 `0001`）同时作为封面和首页。

![Telysta 桌面界面](docs/preview.png)

## 使用

| 平台 | 产物 | 启动方式 |
| --- | --- | --- |
| Windows 10 / 11 x64 | `Telysta-Windows-x64.zip` | 解压后双击 `Telysta.exe` |
| Linux x86_64 | `Telysta-Linux-x86_64.tar.gz` | 解压后运行 `./telysta` |
| macOS 11+，Intel / Apple Silicon | `Telysta-macOS-universal.zip` | 解压后打开 `Telysta.app` |

已构建的本机版本位于 `dist/`，跨平台产物通过 [GitHub Actions](https://github.com/weiserTelysta/manga-to-epub/actions/workflows/build.yml) 获取。实际验证情况见 [验证记录](docs/VALIDATION.md)。

1. 选择文件、文件夹，或直接拖入窗口。
2. 选择输出格式，必要时修改书名和保存位置。
3. 点击「开始制作」，到自动新建的 **Telysta Books** 文件夹中取书。

运行不需要 Rust、Python、Node.js、WebView 或 7-Zip。原始文件保持不变，同名书籍自动编号。

## 输入与页序

支持 ZIP / CBZ / 7Z / RAR / CBR / TAR / TAR.GZ / TGZ，以及 JPG / PNG / WebP / GIF / BMP / TIFF。
多个压缩包各生成一本书；一起选择的单张图片合为一本；文件夹图片递归收集。图片按自然文件名排序，章节目录也参与排序。

`0001 → 0002 → 0010` 不会变成 `1 → 10 → 2`。**第一张图片直接作为封面和第一张阅读页，不额外插入文字封面。**

完整规则、JM 图片包示例、章节编号和格式选择见 [漫画素材指南](docs/MANGA-WORKFLOW.md)。软件处理已经获得的本地图片，不承担站点登录和下载。

## 界面与批量制作

主区展示漫画素材和队列，辅助区设置格式、书名和保存位置。作者、阅读方向、质量与并发数量位于「更多选项」。
默认最多同时制作 **2 本**，可设置 1–8 本；单本内部逐页写入并保持顺序。某一本失败时其余任务继续，停止后会清理未完成文件。

界面支持浅色校园主题、深色校园主题；右键主题按钮可选保留的 Rosé Pine。窗口不足以并排显示时改为上下排列。
封面通过队列缩略图及独立预览查看；错误行可点击查看完整原因。主题和窗口偏好保存在本机。

快捷键：`Ctrl / ⌘ + O` 选择文件，`Ctrl / ⌘ + Shift + O` 选择文件夹，`Ctrl / ⌘ + Enter` 开始制作，`Esc` 关闭说明和预览。

## 命令行

```sh
# 默认 EPUB，图片按自然顺序排列
cargo run --locked --no-default-features --bin telysta-cli -- manga.zip

# 三本并发制作
cargo run --locked --no-default-features --bin telysta-cli -- one.zip two.zip three.7z --jobs 3 --output ./exports

# 手动图片合成一本，设置右向翻页
cargo run --locked --no-default-features --bin telysta-cli -- 0001.jpg 0002.webp 0010.png --title "漫画画册" --direction rtl
```

使用打包版本时替换为 `telysta-cli`，Windows 下为 `telysta-cli.exe`。完整参数见 `--help`。

## 从源码运行与构建

需要 Rust **1.93+** 与平台的 C/C++ 构建工具，首次构建联网下载依赖，运行时离线。

```sh
cargo run --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

平台打包：

```text
Windows: powershell -ExecutionPolicy Bypass -File scripts/build-windows.ps1
Linux:   bash scripts/build-linux.sh
macOS:   bash scripts/build-macos.sh
```

- [平台安装、依赖、Mac universal2 与构建说明](docs/PLATFORMS.md)
- [开发结构、并发实现、测试与 Git 流程](docs/DEVELOPMENT.md)
- [UI 设计系统与约束](docs/UI-DESIGN.md)
- [用户使用说明](docs/使用说明.txt)
- [字体与第三方许可证](THIRD-PARTY-NOTICES.md)

## 格式与边界

EPUB 为固定版式、一图一页，包含封面、目录、阅读顺序及兼容 NCX。正常方向 JPEG / PNG 尽量直接保留，WebP 等转为 JPEG；转码会应用 EXIF 方向，透明区域铺白。PDF 统一使用 RGB JPEG，CBZ 保留图片原始字节。

实体阅读器需要支持 EPUB 固定版式；汉王 N10 自带阅读器和 Kindle 经 Calibre 转换的显示反馈、分页边距修正与对照办法见 [阅读器兼容性说明](docs/READER-COMPATIBILITY.md)。新版输出已显式清除分页边距；设备自身留白和二次转换仍需实机核对。

转码质量默认 90%；WebP 转 JPEG 后体积可能增加。GIF / 多页 TIFF 在 EPUB / PDF 中仅取第一帧 / 第一页。
当前不支持加密包或包内嵌套压缩包。自动忽略隐藏文件、`__MACOSX` 与生成的 `Telysta Books`，不跟随目录中的符号链接。

单张文件限制 128 MiB、单本解压图片总量 8 GiB、最多 50,000 页、单张最多 8,000 万像素。
EPUB / CBZ 逐页写入；PDF 在内存中组装，大图册或低内存设备可以选择 EPUB / CBZ，并将并发数调为 1。

源码仓库：[weiserTelysta/manga-to-epub](https://github.com/weiserTelysta/manga-to-epub)。
