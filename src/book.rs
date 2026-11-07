use crate::{
    input::{
        MAX_PAGE_BYTES, PreparedInput, archive_image_name, check_cancel, extension, natural_cmp,
    },
    model::*,
};
use anyhow::{Context, Result, bail, ensure};
use image::{
    DynamicImage, ImageDecoder, ImageFormat, ImageReader, Rgb, RgbImage, codecs::jpeg::JpegEncoder,
    metadata::Orientation,
};
use lopdf::{Document, Object, Stream, dictionary};
use std::{
    fs::{self, File},
    io::{Cursor, Read, Seek, Write},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use time::{OffsetDateTime, macros::format_description};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

struct Page {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
    extension: &'static str,
    media_type: &'static str,
}

fn read_image(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path).with_context(|| format!("无法打开图片：{}", path.display()))?;
    ensure!(
        file.metadata()?.len() <= MAX_PAGE_BYTES,
        "图片超过 128 MiB：{}",
        path.display()
    );
    let mut bytes = Vec::new();
    file.take(MAX_PAGE_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= MAX_PAGE_BYTES, "图片超过 128 MiB");
    Ok(bytes)
}

fn decode(bytes: &[u8]) -> Result<(DynamicImage, ImageFormat, Orientation)> {
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let format = reader.format().context("无法识别图片内容")?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(512 * 1024 * 1024);
    limits.max_image_width = Some(30_000);
    limits.max_image_height = Some(30_000);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().context("图片损坏或无法解码")?;
    let (width, height) = decoder.dimensions();
    ensure!(
        width > 0 && height > 0 && u64::from(width) * u64::from(height) <= 80_000_000,
        "图片尺寸无效或超过 8000 万像素限制"
    );
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok((image, format, orientation))
}

fn white_rgb(image: DynamicImage) -> RgbImage {
    if !image.color().has_alpha() {
        return image.to_rgb8();
    }
    let rgba = image.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let p = rgba.get_pixel(x, y).0;
        let alpha = u32::from(p[3]);
        Rgb(std::array::from_fn(|i| {
            ((u32::from(p[i]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8
        }))
    })
}

fn prepare_page(path: &Path, options: &ConversionOptions) -> Result<Page> {
    let bytes = read_image(path)?;
    let (image, format, orientation) = decode(&bytes)?;
    let (width, height) = (image.width(), image.height());
    if options.format == BookFormat::Cbz {
        let (ext, media_type) = match format {
            ImageFormat::Jpeg => ("jpg", "image/jpeg"),
            ImageFormat::Png => ("png", "image/png"),
            ImageFormat::WebP => ("webp", "image/webp"),
            ImageFormat::Gif => ("gif", "image/gif"),
            ImageFormat::Bmp => ("bmp", "image/bmp"),
            ImageFormat::Tiff => ("tiff", "image/tiff"),
            _ => bail!("不支持的图片格式"),
        };
        return Ok(Page {
            bytes,
            width,
            height,
            extension: ext,
            media_type,
        });
    }
    if options.format == BookFormat::Epub && orientation == Orientation::NoTransforms {
        match format {
            ImageFormat::Png => {
                return Ok(Page {
                    bytes,
                    width,
                    height,
                    extension: "png",
                    media_type: "image/png",
                });
            }
            ImageFormat::Jpeg => {
                return Ok(Page {
                    bytes,
                    width,
                    height,
                    extension: "jpg",
                    media_type: "image/jpeg",
                });
            }
            _ => {}
        }
    }
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, options.jpeg_quality)
        .encode_image(&white_rgb(image))?;
    Ok(Page {
        bytes: jpeg,
        width,
        height,
        extension: "jpg",
        media_type: "image/jpeg",
    })
}

pub fn xml(value: &str) -> String {
    value
        .chars()
        .filter(|c| matches!(c, '\t' | '\n' | '\r') || *c >= '\u{20}')
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn safe_filename(title: &str) -> String {
    let name: String = title
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(100)
        .collect();
    let name = name.trim().trim_end_matches(['.', ' ']);
    let name = if name.is_empty() {
        "未命名书籍"
    } else {
        name
    };
    let prefix = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    if matches!(prefix.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (prefix.len() == 4
            && (prefix.starts_with("COM") || prefix.starts_with("LPT"))
            && matches!(prefix.as_bytes()[3], b'1'..=b'9'))
    {
        format!("_{name}")
    } else {
        name.to_owned()
    }
}

/// Converts one book, writes atomically, and never replaces existing output.
/// Cancellation or errors remove temporary extraction and output files.
pub fn convert(
    source: &BookSource,
    options: &ConversionOptions,
    cancel: &AtomicBool,
    mut progress: impl FnMut(Progress),
) -> Result<ConversionResult> {
    ensure!(
        (1..=100).contains(&options.jpeg_quality),
        "JPEG 质量必须在 1 到 100 之间"
    );
    ensure!(!options.title.trim().is_empty(), "书名不能为空");
    check_cancel(cancel)?;
    progress(Progress {
        fraction: 0.0,
        message: "正在整理图片…".into(),
        pages_done: 0,
        pages_total: 0,
    });
    let input = PreparedInput::new(source, cancel, |n| {
        progress(Progress {
            fraction: 0.02,
            message: format!("正在解压 · 已读取 {n} 张图片"),
            pages_done: 0,
            pages_total: 0,
        })
    })?;
    let output_dir = options.output_parent.join(OUTPUT_FOLDER);
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("无法创建输出文件夹：{}", output_dir.display()))?;
    let mut temporary = tempfile::NamedTempFile::new_in(&output_dir)
        .context("无法创建输出文件，请检查磁盘空间与权限")?;
    let total = input.pages.len();
    let mut report = |index: usize| {
        progress(Progress {
            fraction: 0.05 + 0.90 * (index as f32 / total as f32),
            message: format!("正在装订 · {index} / {total} 页"),
            pages_done: index,
            pages_total: total,
        })
    };
    match options.format {
        BookFormat::Epub => write_epub(
            temporary.as_file_mut(),
            &input.pages,
            options,
            cancel,
            &mut report,
        )?,
        BookFormat::Cbz => write_cbz(
            temporary.as_file_mut(),
            &input.pages,
            options,
            cancel,
            &mut report,
        )?,
        BookFormat::Pdf => write_pdf(
            temporary.as_file_mut(),
            &input.pages,
            options,
            cancel,
            &mut report,
        )?,
    }
    check_cancel(cancel)?;
    temporary.as_file_mut().flush()?;
    temporary.as_file().sync_all()?;
    let bytes = temporary.as_file().metadata()?.len();
    let base = safe_filename(&options.title);
    let mut suffix = 0_u32;
    let path = loop {
        check_cancel(cancel)?;
        let filename = if suffix == 0 {
            format!("{base}.{}", options.format.extension())
        } else {
            format!("{base} ({suffix}).{}", options.format.extension())
        };
        let path = output_dir.join(filename);
        match temporary.persist_noclobber(&path) {
            Ok(_) => break path,
            Err(err) if err.error.kind() == std::io::ErrorKind::AlreadyExists => {
                temporary = err.file;
                suffix = suffix.checked_add(1).context("输出文件名冲突过多")?;
            }
            Err(err) => return Err(err.error).context("无法保存电子书"),
        }
    };
    progress(Progress {
        fraction: 1.0,
        message: format!("制作完成 · {total} 页"),
        pages_done: total,
        pages_total: total,
    });
    Ok(ConversionResult {
        path,
        pages: total,
        bytes,
    })
}

fn zip_file<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    name: &str,
    bytes: &[u8],
    stored: bool,
) -> Result<()> {
    let method = if stored {
        CompressionMethod::Stored
    } else {
        CompressionMethod::Deflated
    };
    zip.start_file(
        name,
        SimpleFileOptions::default()
            .compression_method(method)
            .unix_permissions(0o644),
    )?;
    zip.write_all(bytes)?;
    Ok(())
}

fn write_cbz(
    file: &mut File,
    paths: &[PathBuf],
    options: &ConversionOptions,
    cancel: &AtomicBool,
    report: &mut impl FnMut(usize),
) -> Result<()> {
    let mut zip = ZipWriter::new(file);
    for (i, path) in paths.iter().enumerate() {
        check_cancel(cancel)?;
        let page = prepare_page(path, options)
            .with_context(|| format!("第 {} 页转换失败：{}", i + 1, path.display()))?;
        zip_file(
            &mut zip,
            &format!("{:05}.{}", i + 1, page.extension),
            &page.bytes,
            true,
        )?;
        report(i + 1);
    }
    zip.finish()?;
    Ok(())
}

fn write_epub(
    file: &mut File,
    paths: &[PathBuf],
    options: &ConversionOptions,
    cancel: &AtomicBool,
    report: &mut impl FnMut(usize),
) -> Result<()> {
    let mut zip = ZipWriter::new(file);
    // OCF: first member, uncompressed, no BOM/newline or extra fields.
    zip_file(&mut zip, "mimetype", b"application/epub+zip", true)?;
    zip_file(&mut zip, "META-INF/container.xml", br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/package.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#, false)?;
    zip_file(
        &mut zip,
        "OEBPS/style.css",
        include_bytes!("epub-page.css"),
        false,
    )?;
    let title = xml(&options.title);
    let author = xml(if options.author.trim().is_empty() {
        "佚名"
    } else {
        &options.author
    });
    let identifier = format!("urn:uuid:{}", uuid::Uuid::new_v4());
    let modified = OffsetDateTime::now_utc().format(format_description!(
        "[year]-[month]-[day]T[hour]:[minute]:[second]Z"
    ))?;
    let mut manifest = String::new();
    let mut spine = String::new();
    let mut nav = String::new();
    let mut ncx = String::new();
    for (i, path) in paths.iter().enumerate() {
        check_cancel(cancel)?;
        let n = i + 1;
        let page = prepare_page(path, options)
            .with_context(|| format!("第 {n} 页转换失败：{}", path.display()))?;
        let image_name = format!("images/page-{n:05}.{}", page.extension);
        let page_name = format!("text/page-{n:05}.xhtml");
        zip_file(&mut zip, &format!("OEBPS/{image_name}"), &page.bytes, true)?;
        let xhtml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="zh-CN" lang="zh-CN"><head><title>{title} · 第 {n} 页</title><meta name="viewport" content="width={w}, height={h}"/><link rel="stylesheet" type="text/css" href="../style.css"/></head><body><img src="../{image_name}" width="{w}" height="{h}" alt="第 {n} 页"/></body></html>"#,
            w = page.width,
            h = page.height
        );
        zip_file(
            &mut zip,
            &format!("OEBPS/{page_name}"),
            xhtml.as_bytes(),
            false,
        )?;
        manifest.push_str(&format!(r#"<item id="image-{n}" href="{image_name}" media-type="{}"{}/><item id="page-{n}" href="{page_name}" media-type="application/xhtml+xml"/>"#, page.media_type, if n == 1 { " properties=\"cover-image\"" } else { "" }));
        spine.push_str(&format!(r#"<itemref idref="page-{n}"/>"#));
        nav.push_str(&format!(r#"<li><a href="{page_name}">第 {n} 页</a></li>"#));
        ncx.push_str(&format!(r#"<navPoint id="nav-{n}" playOrder="{n}"><navLabel><text>第 {n} 页</text></navLabel><content src="{page_name}"/></navPoint>"#));
        report(n);
    }
    let package = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="book-id" xml:lang="zh-CN" prefix="rendition: http://www.idpf.org/vocab/rendition/#"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="book-id">{identifier}</dc:identifier><dc:title>{title}</dc:title><dc:language>zh-CN</dc:language><dc:creator>{author}</dc:creator><meta property="dcterms:modified">{modified}</meta><meta property="rendition:layout">pre-paginated</meta><meta property="rendition:orientation">auto</meta><meta property="rendition:spread">none</meta><meta name="cover" content="image-1"/></metadata><manifest><item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/><item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/><item id="css" href="style.css" media-type="text/css"/>{manifest}</manifest><spine toc="ncx" page-progression-direction="{}">{spine}</spine></package>"#,
        options.direction.as_str()
    );
    zip_file(&mut zip, "OEBPS/package.opf", package.as_bytes(), false)?;
    let navigation = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="zh-CN" lang="zh-CN"><head><title>{title} · 目录</title></head><body><nav epub:type="toc" id="toc"><h1>目录</h1><ol>{nav}</ol></nav><nav epub:type="landmarks" hidden="hidden"><h2>书籍导航</h2><ol><li><a epub:type="cover" href="text/page-00001.xhtml">封面</a></li><li><a epub:type="bodymatter" href="text/page-00001.xhtml">正文</a></li></ol></nav></body></html>"#
    );
    zip_file(&mut zip, "OEBPS/nav.xhtml", navigation.as_bytes(), false)?;
    let ncx = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1" xml:lang="zh-CN"><head><meta name="dtb:uid" content="{identifier}"/><meta name="dtb:depth" content="1"/><meta name="dtb:totalPageCount" content="0"/><meta name="dtb:maxPageNumber" content="0"/></head><docTitle><text>{title}</text></docTitle><navMap>{ncx}</navMap></ncx>"#
    );
    zip_file(&mut zip, "OEBPS/toc.ncx", ncx.as_bytes(), false)?;
    zip.finish()?;
    Ok(())
}

fn pdf_text(value: &str) -> Object {
    let mut bytes = vec![0xfe, 0xff];
    for unit in value.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    Object::string_literal(bytes)
}

fn write_pdf(
    file: &mut File,
    paths: &[PathBuf],
    options: &ConversionOptions,
    cancel: &AtomicBool,
    report: &mut impl FnMut(usize),
) -> Result<()> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        check_cancel(cancel)?;
        let page = prepare_page(path, options)
            .with_context(|| format!("第 {} 页转换失败：{}", i + 1, path.display()))?;
        let image_id = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Image", "Width" => page.width,
                "Height" => page.height, "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
                "Filter" => "DCTDecode",
            },
            page.bytes,
        ));
        let scale = (14_400.0 / page.width.max(page.height) as f64).min(1.0);
        let (w, h) = (page.width as f64 * scale, page.height as f64 * scale);
        let content = format!("q\n{w:.4} 0 0 {h:.4} 0 0 cm\n/Im0 Do\nQ\n").into_bytes();
        let content_id = doc.add_object(Stream::new(dictionary! {}, content));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), Object::Real(w as f32), Object::Real(h as f32)],
            "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image_id } },
            "Contents" => content_id,
        });
        kids.push(page_id.into());
        report(i + 1);
    }
    doc.objects.insert(
        pages_id,
        dictionary! { "Type" => "Pages", "Count" => kids.len() as i64, "Kids" => kids }.into(),
    );
    let catalog = doc.add_object(dictionary! {
        "Type" => "Catalog", "Pages" => pages_id,
        "ViewerPreferences" => dictionary! { "Direction" => if options.direction == Direction::Rtl { "R2L" } else { "L2R" } },
    });
    let info = doc.add_object(dictionary! { "Title" => pdf_text(&options.title), "Author" => pdf_text(&options.author), "Creator" => "Telysta" });
    doc.trailer.set("Root", catalog);
    doc.trailer.set("Info", info);
    check_cancel(cancel)?;
    doc.save_to(file)?;
    Ok(())
}

/// A bounded thumbnail for the GUI. RAR/7Z/TAR preview is deliberately deferred
/// until conversion, so selecting a large solid archive never decompresses it twice.
pub fn preview(source: &BookSource) -> Result<Option<image::RgbaImage>> {
    let bytes = match source {
        BookSource::Images(paths) => read_image(paths.first().context("未选择图片")?)?,
        BookSource::Path(path) if path.is_dir() => {
            let prepared = PreparedInput::new(source, &AtomicBool::new(false), |_| {})?;
            read_image(&prepared.pages[0])?
        }
        BookSource::Path(path) if matches!(extension(path).as_str(), "zip" | "cbz") => {
            let mut archive = zip::ZipArchive::new(File::open(path)?)?;
            let mut entries = Vec::new();
            for i in 0..archive.len() {
                let entry = archive.by_index_raw(i)?;
                let name = entry.name()?;
                if !entry.is_dir() && archive_image_name(&name) {
                    entries.push((name.into_owned(), i));
                }
            }
            entries.sort_by(|a, b| natural_cmp(&a.0, &b.0));
            let Some((_, index)) = entries.first() else {
                return Ok(None);
            };
            let mut entry = archive.by_index(*index)?;
            ensure!(entry.size() <= MAX_PAGE_BYTES, "预览图片过大");
            let mut bytes = Vec::new();
            Read::by_ref(&mut entry)
                .take(MAX_PAGE_BYTES + 1)
                .read_to_end(&mut bytes)?;
            ensure!(bytes.len() as u64 <= MAX_PAGE_BYTES, "预览图片过大");
            bytes
        }
        _ => return Ok(None),
    };
    let (image, _, _) = decode(&bytes)?;
    Ok(Some(image.thumbnail(480, 640).into_rgba8()))
}
