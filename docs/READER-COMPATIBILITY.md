# 实体阅读器与页面留白

Telysta 输出 EPUB 3 固定版式漫画：每张图片独占一页，页面视口与图片尺寸一致，使用 XHTML 的 `img` 元素直接排版。
这要求阅读器支持 EPUB 的 `pre-paginated` 固定版式。支持 EPUB 文字书不等于完整支持这种漫画版式。

## 当前反馈与修正

用户反馈：同一本书在 Readest 中显示正常；汉王 N10 自带阅读器直接打开原始 EPUB 时，每页右侧、底部都有留白；
Kindle 在经 Calibre 转换后、通过 USB 导入的文件上也出现相同现象。这是两条不同的处理路径，不能将 N10 的问题归因于 Calibre。

对照结果：仅补充分页零边距后，用户在 N10 上仍看到很小的页面，几乎缩在右上角；
换成相同图片、相同固定版式元数据、直接使用 `img` 的对照版后，用户实测恢复正常大小。
问题范围因此收窄到原 SVG 页面封装及其尺寸样式在 N10 上的兼容性，不能简单归因于图片本身有白边。

正式输出已采用实测有效的直接图片排版：宽度自适应、高度自动，并限制在页面区域内。
同时使用 `@page` 零边距、正文零边距和零行高，去掉零字号；不在图片文件内添加白边、不裁切图片。
`@page` 的页面框与正文元素是两个层级，清除正文边距不会自动清除分页边距。

**N10 的直接图片对照版已由用户实测恢复正常大小；Kindle 尚未实机复测。**
修正不能强制覆盖阅读器的界面留白、用户设置，也无法保证 Calibre 二次转换后仍保留固定版式。

## 汉王 N10

用新版重新制作一本 EPUB，并以新书名导入，避免继续查看之前的文件或沿用旧书的阅读设置。
在自带阅读器中检查页边距、页眉页脚及原书排版相关选项；具体名称随固件变化。
旧版 SVG 页面请重新制作，单纯重命名旧书不会改变内部排版。
如果新书仍出现固定的右侧、底部空隙，需要结合该固件的实际显示判断是否忽略固定版式或为阅读界面预留空间。
目前没有依据声称所有 N10 固件都完整实现 EPUB 3 固定版式。

可以同时用 Telysta 直接导出同一本 PDF，在 N10 上以原版页面查看作对照。
PDF 中的页面框就是图片尺寸，图片从页面原点铺满整个页面，不经过 EPUB 的文字分页过程。

## Kindle + Calibre + USB

这个流程中，Kindle 阅读的是 Calibre 转换后的文件。Calibre 的页面设置包含输出设备、页面边距，
转换过程中还可能把 SVG 换成普通图片并根据输出设备尺寸缩放；因此不能只检查原始 EPUB 的 CSS。

本地使用 Calibre 9.18.0、`kindle_pw3` 输出配置将对照 EPUB 转为 AZW3：即使源文件分页边距为零，
默认转换生成的 `page_styles.css` 仍设置上下各 `5pt`，转换中间 OPF 也不再包含 EPUB 3 的固定版式元数据。
显式将四侧转换边距设为零后，中间分页 CSS 的上下边距为零。这验证了转换链会重新处理排版，仍不等于 Kindle 实机已经满屏。

```sh
ebook-convert manga.epub manga.azw3 --output-profile kindle_pw3 --margin-left 0 --margin-right 0 --margin-top 0 --margin-bottom 0
```

此处 `kindle_pw3` 是对照实验配置；实际转换应选择自己的设备配置。

若继续使用这条流程，应同时检查 Calibre 的输出格式、输出设备配置和页面边距，并核对转换后文件的版式。
将 Calibre 的四侧边距设为零只能减少它添加的边距，不能保证恢复固定版式漫画。
本工具目前没有 MOBI / AZW3 / KFX 输出，也不声称能够通过普通 Calibre 转换得到 Kindle 满屏漫画。
直接导出 PDF 后用 USB 导入，可以绕开这次 EPUB 再排版；阅读器界面和屏幕比例仍可能产生屏幕留白。

## 正常的等比留白

漫画页与屏幕长宽比不同，为了完整显示原图，阅读器可能在上下或左右留白。
例如 2:3 漫画在 3:4 的显示区域里等比完整显示时，宽度无法同时铺满。
这与额外的分页边距不同；强行同时铺满宽高会造成裁切或拉伸。

## 技术依据

- [W3C EPUB Reading Systems 3.3：固定版式、视口与等比留白](https://www.w3.org/TR/epub-rs-33/#sec-fixed-layout)
- [W3C CSS Paged Media：页面框和 `@page`](https://www.w3.org/TR/css-page-3/#page-model)
- [Calibre 官方手册：Page setup](https://manual.calibre-ebook.com/conversion.html#page-setup)
- [Calibre 转换源码：SVG 替换为普通图片](https://github.com/kovidgoyal/calibre/blob/master/src/calibre/ebooks/oeb/transforms/rasterize.py)
- [Calibre 转换源码：依据设备尺寸及边距缩放图片](https://github.com/kovidgoyal/calibre/blob/master/src/calibre/ebooks/oeb/transforms/rescale.py)
