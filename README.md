# Telysta · 图片成书

用 Rust 编写的桌面图片电子书制作工具。中文 GUI 使用 [Rosé Pine](https://rosepinetheme.com/palette/)
配色，深紫底色、玫瑰色主按钮、鸢尾紫细节，内置中文字体，离线使用。

![Telysta 的 Rosé Pine 中文界面](docs/preview.png)

## 使用

首版 Windows 便携包位于 `dist/Telysta-Windows-x64.zip`（约 11.8 MiB）。
也可以直接运行已经解压好的 `dist/Telysta-Windows-x64/Telysta.exe`。

Windows 10 / 11 x64：解压便携包，双击 **Telysta.exe**。运行时无需安装 Rust、Python、Node.js、WebView 或 7-Zip。

1. 点击「选择文件」或「选择文件夹」，也可以直接拖入窗口。
2. 选择 EPUB（默认）、PDF 或 CBZ；可修改选中书籍的书名、作者、阅读方向和转码质量。
3. 点击「开始制作」。结果保存在自动创建的 **Telysta Books** 文件夹中。

界面提供转换队列、ZIP / 图片文件夹封面预览、逐页进度、停止制作、错误提示、重试以及打开输出目录。
源文件不会被修改；已存在的书籍不会被覆盖，同名结果自动添加 ` (1)`、` (2)`。

## 输入与成书规则

| 选择内容 | 结果 |
| --- | --- |
| 一个或多个 ZIP / CBZ / 7Z / RAR / CBR / TAR / TAR.GZ / TGZ | 每个压缩包各生成一本书 |
| 一起选择多张 JPG / JPEG / PNG / WebP / GIF / BMP / TIFF | 合并成一本书 |
| 图片文件夹 | 递归收集子文件夹中的图片，合成一本书 |
| 含多个压缩包的文件夹 | 每个压缩包各生成一本书 |
| 同时含图片与压缩包的文件夹 | 松散图片合成一本，压缩包各自成书 |

文件名按自然顺序排序：`1 → 2 → 10`。含章节目录时按完整相对路径排序。
自动忽略隐藏目录、`__MACOSX` 和 `Telysta Books`，不跟随文件夹内的符号链接。
不自动展开压缩包内的嵌套压缩包。

默认在每份源文件的父目录创建 `Telysta Books`。选择「更改位置」后，在选定目录下创建
同名输出文件夹，所有书籍集中保存。单独选中的图片合为一本，其位置以排序后第一张图片为准。

## 输出格式

| 格式 | 行为 |
| --- | --- |
| **EPUB** | EPUB 3 固定版式，一图一页、保留比例，含封面、目录、阅读顺序及 NCX 兼容目录 |
| **PDF** | 每张图片为一页，按原始长宽比嵌入，支持中文标题与作者元数据 |
| **CBZ** | 按顺序重命名打包，保留图片原始字节，适合漫画阅读器 |

EPUB 中正常方向的 JPEG / PNG 直接保留，避免重复编码；WebP、BMP、GIF、TIFF 等转为 JPEG。
转码会应用 EXIF 方向，透明区域以白色铺底。PDF 图片统一转为 RGB JPEG。
转码质量默认 90%，GUI 可选 60–100%，CLI 可选 1–100%。
WebP 转成 JPEG 后体积可能增加；希望保留原始图片体积时可选择 CBZ。
EPUB / PDF 对动画 GIF 和多页 TIFF 仅取第一帧 / 第一页。CBZ 保留完整原始文件。
固定版式 EPUB 的显示效果取决于阅读器对 EPUB 3 固定版式的支持。

## 运行源码

需要 **Rust 1.93+**；首次构建需要联网下载依赖，此后程序运行完全离线。

```sh
cargo run --locked
```

也可以给 GUI 传入初始文件：

```sh
cargo run --locked --bin telysta -- "1487614.zip"
```

Windows 构建需要 Visual Studio Build Tools 的「使用 C++ 的桌面开发」组件与 Windows SDK。
Linux 构建需要 C/C++ 编译器；GUI 使用 X11 / OpenGL，文件选择框由系统桌面服务提供。

## 打包 Windows 可执行文件

在 Windows PowerShell 中执行：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\build-windows.ps1
```

脚本执行格式检查、测试和 Release 构建，产出：

```text
dist/
  Telysta-Windows-x64/
    Telysta.exe
    telysta-cli.exe
    README.md
    LICENSE
    THIRD-PARTY-NOTICES.md
    FONT-LICENSE.txt
    UNRAR-LICENSE.txt
  Telysta-Windows-x64.zip
```

GUI 字体、图标和解压逻辑均内置于可执行文件；MSVC 构建静态链接 C 运行库。
`.github/workflows/windows.yml` 提供 GitHub Actions 的 Windows 构建流程，可手动触发，
也会在 `v*` 标签推送或相关 Pull Request 时运行，产物出现在 Actions 的 Artifacts 中。

## 命令行

```sh
# 默认 EPUB；自动创建源目录下的 Telysta Books
cargo run --locked --no-default-features --bin telysta-cli -- "1487614.zip"

# PDF，指定输出父目录和书名
cargo run --locked --no-default-features --bin telysta-cli -- ./pictures --format pdf --output ./exports --title "旅行手记"

# 多个压缩包，保留原始文件名
cargo run --locked --no-default-features --bin telysta-cli -- one.zip two.7z --format cbz

# 单张图片合成一本，设置从右至左翻页
cargo run --locked --no-default-features --bin telysta-cli -- 1.png 2.webp 10.jpg --direction rtl --title "画册"
```

已打包时，将 `cargo run ... --` 替换为 `telysta-cli.exe` 即可。完整参数：`telysta-cli.exe --help`。
批量转换时一项失败会继续处理其余项目，最终返回非零退出码；`--title` 仅适用于一本书。

## 测试与项目结构

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

测试覆盖 EPUB 容器与 XML、自然页序、WebP 转码、原图保留、PDF 页数与元数据、
ZIP / 7Z / RAR / TAR.GZ、目录发现、文件去重、取消清理、失败清理、防路径穿越与同名文件保护。

```text
src/ui.rs          Rosé Pine GUI、后台任务、进度与封面预览
src/input.rs       输入发现、自然排序、临时解压与体积限制
src/book.rs        EPUB / PDF / CBZ 编写、图片处理、原子保存
src/model.rs       转换参数与事件数据
src/bin/cli.rs     可单独构建的命令行入口
tests/            转换集成测试
scripts/          Windows 打包、图标生成
assets/           内嵌字体、图标、Windows 清单与许可证
```

压缩包中的图片使用程序生成的临时文件名，原始归档路径不会写入文件系统。
单张压缩图片限制 128 MiB、单本解压图片总量 8 GiB、最多 50,000 页、
单张图片最多 8,000 万像素。EPUB / CBZ 逐页写入；PDF 由文档库在内存中组装，
大图册建议选择 EPUB / CBZ。暂不支持加密压缩包，分卷 RAR 需要全部分卷位于同一目录。

字体授权与解压库说明见 [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)。
UnRAR 的跨平台构建补丁保存在 `vendor/unrar_sys`，具体变更见该目录的 `TELYSTA-PATCH.md`。
