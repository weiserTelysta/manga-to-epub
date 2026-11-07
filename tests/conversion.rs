use image::{ImageFormat, Rgb, RgbImage};
use std::{
    cmp::Ordering,
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering as AtomicOrdering},
};
use telysta::{
    BookFormat, BookSource, ConversionOptions, Direction, convert, discover, input::natural_cmp,
};
use tempfile::TempDir;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

fn png(color: [u8; 3]) -> Vec<u8> {
    let mut data = Cursor::new(Vec::new());
    RgbImage::from_pixel(12, 18, Rgb(color))
        .write_to(&mut data, ImageFormat::Png)
        .unwrap();
    data.into_inner()
}

fn archive(path: &Path, entries: &[(&str, Vec<u8>)]) {
    let mut writer = ZipWriter::new(File::create(path).unwrap());
    for (name, data) in entries {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap();
}

fn fixture() -> (TempDir, BookSource, ConversionOptions) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("测试 & 漫画.zip");
    archive(
        &path,
        &[
            ("10.png", png([0, 0, 255])),
            ("2.png", png([0, 255, 0])),
            ("1.png", png([255, 0, 0])),
            ("__MACOSX/._1.png", b"junk".to_vec()),
            ("readme.txt", b"ignored".to_vec()),
        ],
    );
    let source = BookSource::Path(path);
    let mut options = ConversionOptions::for_source(&source);
    options.title = "书名 <一> & \"二\"".into();
    options.author = "作者 & 合作".into();
    (dir, source, options)
}

fn zip_text(zip: &mut ZipArchive<File>, name: &str) -> String {
    let mut value = String::new();
    zip.by_name(name)
        .unwrap()
        .read_to_string(&mut value)
        .unwrap();
    value
}

#[test]
fn epub_has_valid_xml_metadata_navigation_cover_and_natural_page_order() {
    let (_dir, source, mut options) = fixture();
    options.direction = Direction::Rtl;
    let result = convert(&source, &options, &AtomicBool::new(false), |_| {}).unwrap();
    assert_eq!(result.pages, 3);
    assert_eq!(result.path.extension().unwrap(), "epub");
    let mut zip = ZipArchive::new(File::open(result.path).unwrap()).unwrap();
    let first = zip.by_index(0).unwrap();
    assert_eq!(first.name().unwrap(), "mimetype");
    assert_eq!(first.compression(), CompressionMethod::Stored);
    drop(first);
    assert_eq!(zip_text(&mut zip, "mimetype"), "application/epub+zip");
    let opf = zip_text(&mut zip, "OEBPS/package.opf");
    assert!(opf.contains("书名 &lt;一&gt; &amp; &quot;二&quot;"));
    assert!(opf.contains("page-progression-direction=\"rtl\""));
    assert!(opf.contains("properties=\"cover-image\""));
    assert_eq!(opf.matches("<itemref ").count(), 3);
    assert!(zip_text(&mut zip, "OEBPS/nav.xhtml").contains("text/page-00003.xhtml"));
    for (n, expected) in [[255, 0, 0], [0, 255, 0], [0, 0, 255]].iter().enumerate() {
        let mut bytes = Vec::new();
        zip.by_name(&format!("OEBPS/images/page-{:05}.png", n + 1))
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(
            image::load_from_memory(&bytes)
                .unwrap()
                .to_rgb8()
                .get_pixel(0, 0)
                .0,
            *expected
        );
    }
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap();
        let name = entry.name().unwrap().into_owned();
        if [".xml", ".xhtml", ".opf", ".ncx"]
            .iter()
            .any(|ext| name.ends_with(ext))
        {
            let mut text = String::new();
            entry.read_to_string(&mut text).unwrap();
            let mut reader = quick_xml::Reader::from_str(&text);
            loop {
                match reader
                    .read_event()
                    .unwrap_or_else(|e| panic!("{name}: {e}"))
                {
                    quick_xml::events::Event::Eof => break,
                    quick_xml::events::Event::Start(tag) | quick_xml::events::Event::Empty(tag) => {
                        for attr in tag.attributes() {
                            attr.unwrap();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[test]
fn webp_is_converted_for_epub_but_cbz_preserves_original_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let mut data = Cursor::new(Vec::new());
    RgbImage::from_pixel(20, 30, Rgb([150, 60, 80]))
        .write_to(&mut data, ImageFormat::WebP)
        .unwrap();
    let data = data.into_inner();
    let path = dir.path().join("page.webp");
    fs::write(&path, &data).unwrap();
    let source = BookSource::Images(vec![path]);
    let mut options = ConversionOptions::for_source(&source);
    let result = convert(&source, &options, &AtomicBool::new(false), |_| {}).unwrap();
    let mut zip = ZipArchive::new(File::open(result.path).unwrap()).unwrap();
    assert!(zip.by_name("OEBPS/images/page-00001.jpg").is_ok());
    options.format = BookFormat::Cbz;
    let result = convert(&source, &options, &AtomicBool::new(false), |_| {}).unwrap();
    let mut zip = ZipArchive::new(File::open(result.path).unwrap()).unwrap();
    let mut actual = Vec::new();
    zip.by_name("00001.webp")
        .unwrap()
        .read_to_end(&mut actual)
        .unwrap();
    assert_eq!(actual, data);
}

#[test]
fn pdf_is_readable_and_keeps_page_dimensions_and_chinese_title() {
    let (_dir, source, mut options) = fixture();
    options.format = BookFormat::Pdf;
    let result = convert(&source, &options, &AtomicBool::new(false), |_| {}).unwrap();
    let doc = lopdf::Document::load(result.path).unwrap();
    let pages = doc.get_pages();
    assert_eq!(pages.len(), 3);
    let page = doc
        .get_object(*pages.get(&1).unwrap())
        .unwrap()
        .as_dict()
        .unwrap();
    let media = page.get(b"MediaBox").unwrap().as_array().unwrap();
    assert_eq!(media[2].as_float().unwrap(), 12.0);
    assert_eq!(media[3].as_float().unwrap(), 18.0);
    let info = doc
        .get_object(doc.trailer.get(b"Info").unwrap().as_reference().unwrap())
        .unwrap()
        .as_dict()
        .unwrap();
    let title = info.get(b"Title").unwrap().as_str().unwrap();
    assert!(title.starts_with(&[0xfe, 0xff]));
    let decoded = String::from_utf16(
        &title[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u16::from_be_bytes([p[0], p[1]]))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(decoded, options.title);
}

#[test]
fn duplicate_output_is_numbered_without_overwriting() {
    let (_dir, source, options) = fixture();
    let a = convert(&source, &options, &AtomicBool::new(false), |_| {}).unwrap();
    let original = fs::read(&a.path).unwrap();
    let b = convert(&source, &options, &AtomicBool::new(false), |_| {}).unwrap();
    assert_ne!(a.path, b.path);
    assert!(
        b.path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .ends_with(" (1)")
    );
    assert_eq!(fs::read(a.path).unwrap(), original);
}

#[test]
fn cancellation_mid_conversion_leaves_no_partial_book() {
    let (_dir, source, options) = fixture();
    let cancel = AtomicBool::new(false);
    let result = convert(&source, &options, &cancel, |p| {
        if p.pages_done == 1 {
            cancel.store(true, AtomicOrdering::Relaxed);
        }
    });
    assert!(result.unwrap_err().to_string().contains("已取消"));
    let folder = options.output_parent.join("Telysta Books");
    assert_eq!(fs::read_dir(folder).unwrap().count(), 0);
}

#[test]
fn corrupt_image_fails_with_page_context_and_removes_temporary_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broken.png");
    fs::write(&path, b"not an image").unwrap();
    let source = BookSource::Images(vec![path]);
    let options = ConversionOptions::for_source(&source);
    let error = convert(&source, &options, &AtomicBool::new(false), |_| {}).unwrap_err();
    assert!(error.to_string().contains("第 1 页转换失败"));
    assert_eq!(
        fs::read_dir(options.output_parent.join("Telysta Books"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn archive_path_traversal_entries_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unsafe.zip");
    archive(
        &path,
        &[
            ("../escape.png", png([1, 2, 3])),
            ("C:\\escape.png", png([1, 2, 3])),
            ("./safe.png", png([4, 5, 6])),
        ],
    );
    let source = BookSource::Path(path);
    let options = ConversionOptions::for_source(&source);
    let result = convert(&source, &options, &AtomicBool::new(false), |_| {}).unwrap();
    assert_eq!(result.pages, 1);
    assert!(!dir.path().join("escape.png").exists());
}

#[test]
fn folder_discovery_separates_archives_and_excludes_generated_books() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("chapter")).unwrap();
    fs::write(dir.path().join("chapter/1.png"), png([1, 2, 3])).unwrap();
    let compressed = dir.path().join("archive.zip");
    archive(&compressed, &[("1.png", png([1, 2, 3]))]);
    fs::create_dir(dir.path().join("Telysta Books")).unwrap();
    archive(
        &dir.path().join("Telysta Books/old.cbz"),
        &[("1.png", png([1, 2, 3]))],
    );
    let sources = discover(vec![dir.path().to_owned(), compressed]).unwrap();
    assert_eq!(sources.len(), 2);
    let source = sources.iter().find(|s| s.path().is_dir()).unwrap();
    let result = convert(
        source,
        &ConversionOptions::for_source(source),
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    assert_eq!(result.pages, 1);
}

#[test]
fn single_images_are_grouped_and_deduplicated() {
    let dir = tempfile::tempdir().unwrap();
    let paths: Vec<PathBuf> = ["10.png", "2.png", "1.png"]
        .iter()
        .map(|n| {
            let path = dir.path().join(n);
            fs::write(&path, png([1, 2, 3])).unwrap();
            path
        })
        .collect();
    let mut inputs = paths.clone();
    inputs.push(paths[0].clone());
    let sources = discover(inputs).unwrap();
    assert_eq!(sources.len(), 1);
    let BookSource::Images(images) = &sources[0] else {
        panic!("expected images")
    };
    assert_eq!(images.len(), 3);
    assert_eq!(images[0].file_name().unwrap(), "1.png");
    assert_eq!(images[1].file_name().unwrap(), "2.png");
}

#[test]
fn tar_gz_archive_is_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pages.tar.gz");
    let gz =
        flate2::write::GzEncoder::new(File::create(&path).unwrap(), flate2::Compression::default());
    let mut tar = tar::Builder::new(gz);
    let image = png([1, 2, 3]);
    let mut header = tar::Header::new_gnu();
    header.set_size(image.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, "sub/1.png", image.as_slice())
        .unwrap();
    tar.into_inner().unwrap().finish().unwrap();
    let source = BookSource::Path(path);
    assert_eq!(
        convert(
            &source,
            &ConversionOptions::for_source(&source),
            &AtomicBool::new(false),
            |_| {}
        )
        .unwrap()
        .pages,
        1
    );
}

#[test]
fn sevenz_archive_is_supported() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("images");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("2.png"), png([0, 255, 0])).unwrap();
    fs::write(folder.join("1.png"), png([255, 0, 0])).unwrap();
    let path = dir.path().join("pages.7z");
    sevenz_rust2::compress_to_path(&folder, &path).unwrap();
    let source = BookSource::Path(path);
    assert_eq!(
        convert(
            &source,
            &ConversionOptions::for_source(&source),
            &AtomicBool::new(false),
            |_| {}
        )
        .unwrap()
        .pages,
        2
    );
}

#[test]
fn natural_sort_handles_nested_paths_case_and_large_page_numbers() {
    assert_eq!(
        natural_cmp("chapter2/10.png", "chapter10/1.png"),
        Ordering::Less
    );
    assert_eq!(natural_cmp("PAGE2.PNG", "page10.png"), Ordering::Less);
    assert_eq!(
        natural_cmp("999999999999999999999.png", "1000000000000000000000.png"),
        Ordering::Less
    );
    assert_eq!(natural_cmp("2.png", "00010.png"), Ordering::Less);
    assert_eq!(natural_cmp("0.png", "1.png"), Ordering::Less);
}

#[test]
fn rar_archive_is_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("图片归档.rar");
    fs::write(&path, include_bytes!("fixtures/page.rar")).unwrap();
    let source = BookSource::Path(path);
    let options = ConversionOptions::for_source(&source);
    let result = convert(&source, &options, &AtomicBool::new(false), |_| {}).unwrap();
    assert_eq!(result.pages, 1);
    let mut zip = ZipArchive::new(File::open(result.path).unwrap()).unwrap();
    let mut bytes = Vec::new();
    zip.by_name("OEBPS/images/page-00001.png")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(
        image::load_from_memory(&bytes)
            .unwrap()
            .to_rgb8()
            .get_pixel(0, 0)
            .0,
        [255, 0, 0]
    );
}

#[test]
fn empty_inputs_and_invalid_options_fail_clearly() {
    let dir = tempfile::tempdir().unwrap();
    assert!(discover(vec![dir.path().to_owned()]).is_err());
    let (_fixture, source, mut options) = fixture();
    options.jpeg_quality = 0;
    assert!(convert(&source, &options, &AtomicBool::new(false), |_| {}).is_err());
    options.jpeg_quality = 90;
    options.title = "  ".into();
    assert!(convert(&source, &options, &AtomicBool::new(false), |_| {}).is_err());
}

#[test]
fn filenames_are_valid_on_windows() {
    use telysta::book::safe_filename;
    assert_eq!(safe_filename("CON"), "_CON");
    assert_eq!(safe_filename("LPT1.txt"), "_LPT1.txt");
    assert_eq!(safe_filename("名字/章节:*?"), "名字_章节___");
    assert_eq!(safe_filename("..."), "未命名书籍");
    assert_eq!(safe_filename("book. "), "book");
}

#[test]
fn numbered_first_image_is_both_the_cover_and_the_first_reading_page() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("漫画图片包.zip");
    archive(
        &path,
        &[
            ("漫画/0003.png", png([0, 0, 255])),
            ("漫画/0001.png", png([255, 0, 0])),
            ("漫画/0002.png", png([0, 255, 0])),
        ],
    );
    let source = BookSource::Path(path);
    let result = convert(
        &source,
        &ConversionOptions::for_source(&source),
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    let mut zip = ZipArchive::new(File::open(result.path).unwrap()).unwrap();
    let opf = zip_text(&mut zip, "OEBPS/package.opf");
    assert!(opf.contains("id=\"image-1\" href=\"images/page-00001.png\" media-type=\"image/png\" properties=\"cover-image\""));
    assert!(opf.contains(
        "<spine toc=\"ncx\" page-progression-direction=\"ltr\"><itemref idref=\"page-1\"/>"
    ));
    assert!(zip_text(&mut zip, "OEBPS/text/page-00001.xhtml").contains("../images/page-00001.png"));
    let mut bytes = Vec::new();
    zip.by_name("OEBPS/images/page-00001.png")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(
        image::load_from_memory(&bytes)
            .unwrap()
            .to_rgb8()
            .get_pixel(0, 0)
            .0,
        [255, 0, 0]
    );
}

#[test]
fn epub_pages_have_no_publisher_margins_and_match_each_original_image() {
    let dir = tempfile::tempdir().unwrap();
    let mut paths = Vec::new();
    let dimensions = [(600, 900), (900, 600), (800, 800)];
    for (i, (width, height)) in dimensions.iter().copied().enumerate() {
        let path = dir.path().join(format!("{i:04}.png"));
        RgbImage::from_pixel(width, height, Rgb([30, 60, 90]))
            .save(&path)
            .unwrap();
        paths.push(path);
    }
    let source = BookSource::Images(paths.clone());
    let result = convert(
        &source,
        &ConversionOptions::for_source(&source),
        &AtomicBool::new(false),
        |_| {},
    )
    .unwrap();
    let mut zip = ZipArchive::new(File::open(result.path).unwrap()).unwrap();
    let css = zip_text(&mut zip, "OEBPS/style.css");
    assert!(css.contains("@page {\n    margin: 0;\n    padding: 0;\n}"));
    assert!(css.contains("margin: 0 !important;"));
    assert!(css.contains("line-height: 0;"));
    assert!(
        zip_text(&mut zip, "OEBPS/package.opf")
            .contains("<meta property=\"rendition:layout\">pre-paginated</meta>")
    );
    for (i, (width, height)) in dimensions.iter().enumerate() {
        let n = i + 1;
        let xhtml = zip_text(&mut zip, &format!("OEBPS/text/page-{n:05}.xhtml"));
        let mut reader = quick_xml::Reader::from_str(&xhtml);
        let mut checked = 0;
        loop {
            match reader.read_event().unwrap() {
                quick_xml::events::Event::Eof => break,
                quick_xml::events::Event::Start(tag) | quick_xml::events::Event::Empty(tag) => {
                    let attr = |name| {
                        tag.try_get_attribute(name)
                            .unwrap()
                            .unwrap()
                            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .unwrap()
                            .into_owned()
                    };
                    match tag.name().as_ref() {
                        b"meta" => {
                            assert_eq!(attr("name"), "viewport");
                            assert_eq!(attr("content"), format!("width={width}, height={height}"));
                            checked += 1;
                        }
                        b"svg" => {
                            assert_eq!(attr("viewBox"), format!("0 0 {width} {height}"));
                            assert_eq!(attr("preserveAspectRatio"), "xMidYMid meet");
                            checked += 1;
                        }
                        b"image" => {
                            assert_eq!(attr("x"), "0");
                            assert_eq!(attr("y"), "0");
                            assert_eq!(attr("width"), width.to_string());
                            assert_eq!(attr("height"), height.to_string());
                            checked += 1;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        assert_eq!(checked, 3);
        let mut bytes = Vec::new();
        zip.by_name(&format!("OEBPS/images/page-{n:05}.png"))
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        // Layout fixes must not add white pixels, crop or stretch source images.
        assert_eq!(bytes, fs::read(&paths[i]).unwrap());
    }
}
