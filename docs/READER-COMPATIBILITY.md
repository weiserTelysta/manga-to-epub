# 实体阅读器与页面留白

Telysta 输出 EPUB 3 固定版式漫画：每张图片独占一页，页面尺寸、SVG 画布和图片尺寸一致。
这要求阅读器支持 EPUB 的 `pre-paginated` 固定版式。支持 EPUB 文字书不等于完整支持这种漫画版式。

## 当前反馈与修正

用户反馈：同一本书在 Readest 中显示正常；汉王 N10 自带阅读器直接打开原始 EPUB 时，每页右侧、底部都有留白；
Kindle 在经 Calibre 转换后、通过 USB 导入的文件上也出现相同现象。这是两条不同的处理路径，不能将 N10 的问题归因于 Calibre。

此前页面 CSS 只将 `html` / `body` 的边距设为零，未显式声明分页页面框的边距。
`@page` 的页面框与正文元素是两个层级，清除正文边距不会自动清除分页边距。
现已补充 `@page` 零边距、零内边距，以及图片页零行高、零字号；SVG 显式保持比例、图片从 `(0,0)` 开始。
这修正了输出端的兼容性缺口，不裁切、拉伸图片，也不会在图片文件内添加白边。

**尚未在汉王 N10 或 Kindle 实机复测。** 这些修正无法强制覆盖阅读器自身的界面留白、用户排版设置，
也无法保证 Calibre 二次转换后仍保留固定版式。不要把 EPUBCheck 通过当作实体阅读器显示已经通过。

## 汉王 N10

用新版重新制作一本 EPUB，并以新书名导入，避免继续查看之前的文件或沿用旧书的阅读设置。
在自带阅读器中检查页边距、页眉页脚及原书排版相关选项；具体名称随固件变化。
如果仍然出现固定的右侧、底部空隙，需要结合该固件的实际显示判断是否忽略固定版式或为阅读界面预留空间。
目前没有依据声称所有 N10 固件都完整实现 EPUB 3 固定版式。

可以同时用 Telysta 直接导出同一本 PDF，在 N10 上以原版页面查看作对照。
PDF 中的页面框就是图片尺寸，图片从页面原点铺满整个页面，不经过 EPUB 的文字分页过程。

## Kindle + Calibre + USB

这个流程中，Kindle 阅读的是 Calibre 转换后的文件。Calibre 的页面设置包含输出设备、页面边距，
转换过程中还可能把 SVG 换成普通图片并根据输出设备尺寸缩放；因此不能只检查原始 EPUB 的 CSS。

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
