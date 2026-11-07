use crate::model::{BookSource, OUTPUT_FOLDER};
use anyhow::{Context, Result, bail, ensure};
use std::{
    cmp::Ordering,
    collections::HashSet,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Mutex, TryLockError,
        atomic::{AtomicBool, Ordering as AtomicOrdering},
    },
};
use tempfile::TempDir;
use walkdir::WalkDir;

pub const MAX_PAGE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_PAGES: usize = 50_000;
// UnRAR contains process-global state. Serialize its extraction stage while
// allowing other archive types and all book-writing stages to run in parallel.
static RAR_LOCK: Mutex<()> = Mutex::new(());

pub fn extension(path: &Path) -> String {
    path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase()
}

pub fn is_image(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tif" | "tiff"
    )
}

pub fn is_archive(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "zip" | "cbz" | "7z" | "rar" | "cbr" | "tar" | "tgz"
    ) || path
        .to_string_lossy()
        .to_ascii_lowercase()
        .ends_with(".tar.gz")
}

fn visible(path: &Path) -> bool {
    !path.components().any(|part| {
        let std::path::Component::Normal(name) = part else {
            return false;
        };
        let name = name.to_string_lossy();
        name == "__MACOSX" || name.starts_with('.') || name == OUTPUT_FOLDER
    })
}

fn walk(path: &Path) -> impl Iterator<Item = Result<walkdir::DirEntry>> {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| entry.depth() == 0 || visible(Path::new(entry.file_name())))
        .map(|entry| entry.context("无法读取文件夹，请检查访问权限"))
}

/// Folders become one book of loose images (including subfolders). Archives
/// found in a selected folder each become their own book. Explicit images
/// selected together become one book. Inputs are canonicalized and deduplicated.
pub fn discover(paths: Vec<PathBuf>) -> Result<Vec<BookSource>> {
    let mut sources = Vec::new();
    let mut images = Vec::new();
    let mut seen = HashSet::new();
    for path in paths {
        let path =
            fs::canonicalize(&path).with_context(|| format!("找不到文件：{}", path.display()))?;
        if !seen.insert(path.clone()) {
            continue;
        }
        if path.is_dir() {
            let mut has_images = false;
            let mut archives = Vec::new();
            for entry in walk(&path) {
                let entry = entry?;
                if !entry.file_type().is_file() {
                    continue;
                }
                if is_image(entry.path()) {
                    has_images = true;
                } else if is_archive(entry.path()) {
                    archives.push(entry.into_path());
                }
            }
            if has_images {
                sources.push(BookSource::Path(path.clone()));
            }
            archives.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
            sources.extend(archives.into_iter().map(BookSource::Path));
            ensure!(
                has_images || sources.iter().any(|s| s.path().starts_with(&path)),
                "文件夹内没有支持的图片或压缩包：{}",
                path.display()
            );
        } else if is_image(&path) {
            images.push(path);
        } else if is_archive(&path) {
            sources.push(BookSource::Path(path));
        } else {
            bail!(
                "不支持的文件：{}。请选择图片、ZIP、7Z、RAR 或 TAR 压缩包。",
                path.display()
            );
        }
    }
    if !images.is_empty() {
        images.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
        sources.push(BookSource::Images(images));
    }
    let mut unique = Vec::new();
    for source in sources {
        if !unique.contains(&source) {
            unique.push(source);
        }
    }
    Ok(unique)
}

pub fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    ensure!(!cancel.load(AtomicOrdering::Relaxed), "已取消转换");
    Ok(())
}

pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let aa = a.to_lowercase();
    let bb = b.to_lowercase();
    let (a_bytes, b_bytes) = (aa.as_bytes(), bb.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < a_bytes.len() && j < b_bytes.len() {
        if a_bytes[i].is_ascii_digit() && b_bytes[j].is_ascii_digit() {
            let (start_a, start_b) = (i, j);
            while i < a_bytes.len() && a_bytes[i].is_ascii_digit() {
                i += 1;
            }
            while j < b_bytes.len() && b_bytes[j].is_ascii_digit() {
                j += 1;
            }
            let da = aa[start_a..i].trim_start_matches('0');
            let db = bb[start_b..j].trim_start_matches('0');
            let ord = da.len().cmp(&db.len()).then_with(|| da.cmp(db));
            if ord != Ordering::Equal {
                return ord;
            }
        } else {
            let ord = a_bytes[i].cmp(&b_bytes[j]);
            if ord != Ordering::Equal {
                return ord;
            }
            i += 1;
            j += 1;
        }
    }
    (a_bytes.len() - i)
        .cmp(&(b_bytes.len() - j))
        .then_with(|| a.cmp(b))
}

pub struct PreparedInput {
    pub pages: Vec<PathBuf>,
    _staging: Option<TempDir>,
}

impl PreparedInput {
    pub fn new(
        source: &BookSource,
        cancel: &AtomicBool,
        mut on_extract: impl FnMut(usize),
    ) -> Result<Self> {
        check_cancel(cancel)?;
        let (mut pages, staging) = match source {
            BookSource::Images(images) => {
                ensure!(!images.is_empty(), "未选择图片");
                (images.clone(), None)
            }
            BookSource::Path(path) if path.is_dir() => {
                let mut pages = Vec::new();
                for entry in walk(path) {
                    check_cancel(cancel)?;
                    let entry = entry?;
                    if entry.file_type().is_file() && is_image(entry.path()) {
                        pages.push(entry.into_path());
                    }
                    ensure!(pages.len() <= MAX_PAGES, "图片数量超过 {MAX_PAGES} 页限制");
                }
                (pages, None)
            }
            BookSource::Path(path) => {
                let staging = tempfile::tempdir().context("无法创建解压临时目录")?;
                let mut sink = Extractor {
                    dir: staging.path(),
                    entries: Vec::new(),
                    total: 0,
                    cancel,
                    on_extract: &mut on_extract,
                };
                match extension(path).as_str() {
                    "zip" | "cbz" => extract_zip(path, &mut sink)?,
                    "7z" => extract_7z(path, &mut sink)?,
                    "rar" | "cbr" => extract_rar(path, &mut sink)?,
                    "tar" => extract_tar(File::open(path)?, &mut sink)?,
                    "gz" | "tgz" => {
                        extract_tar(flate2::read::GzDecoder::new(File::open(path)?), &mut sink)?
                    }
                    _ => bail!("不支持的压缩格式：{}", path.display()),
                }
                sink.entries.sort_by(|a, b| natural_cmp(&a.0, &b.0));
                let pages = sink.entries.into_iter().map(|(_, path)| path).collect();
                (pages, Some(staging))
            }
        };
        if staging.is_none() {
            pages.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
            pages.dedup();
        }
        ensure!(
            !pages.is_empty(),
            "没有找到支持的图片（JPG、PNG、WebP、GIF、BMP、TIFF）"
        );
        ensure!(pages.len() <= MAX_PAGES, "图片数量超过 {MAX_PAGES} 页限制");
        Ok(Self {
            pages,
            _staging: staging,
        })
    }
}

pub(crate) fn archive_image_name(name: &str) -> bool {
    let normalized = name.replace('\\', "/");
    let path = Path::new(&normalized);
    is_image(path)
        && visible(path)
        && !normalized.starts_with('/')
        && !normalized.contains(':')
        && !normalized.split('/').any(|p| p == "..")
}

struct Extractor<'a> {
    dir: &'a Path,
    entries: Vec<(String, PathBuf)>,
    total: u64,
    cancel: &'a AtomicBool,
    on_extract: &'a mut dyn FnMut(usize),
}

impl Extractor<'_> {
    fn push(&mut self, name: &str, size: u64, reader: &mut dyn Read) -> Result<()> {
        check_cancel(self.cancel)?;
        ensure!(size <= MAX_PAGE_BYTES, "图片过大（上限 128 MiB）：{name}");
        ensure!(
            self.entries.len() < MAX_PAGES,
            "图片数量超过 {MAX_PAGES} 页限制"
        );
        ensure!(
            self.total.saturating_add(size) <= MAX_TOTAL_BYTES,
            "解压图片总量超过 8 GiB 限制"
        );
        // The archive name is NEVER used as a filesystem path.
        let path = self.dir.join(format!(
            "{:06}.{}",
            self.entries.len(),
            extension(Path::new(name))
        ));
        let mut output = File::create(&path)?;
        let remaining = (MAX_TOTAL_BYTES - self.total).min(MAX_PAGE_BYTES);
        let copied = io::copy(&mut reader.take(remaining + 1), &mut output)
            .with_context(|| format!("解压失败：{name}"))?;
        ensure!(copied <= remaining, "图片解压大小超过限制：{name}");
        output.flush()?;
        self.total += copied;
        self.entries.push((name.replace('\\', "/"), path));
        (self.on_extract)(self.entries.len());
        Ok(())
    }
}

fn extract_zip(path: &Path, sink: &mut Extractor<'_>) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(path)?)
        .context("无法读取 ZIP：文件损坏、加密或格式不受支持")?;
    for index in 0..archive.len() {
        check_cancel(sink.cancel)?;
        let entry = archive.by_index_raw(index)?;
        let name = entry.name()?.into_owned();
        if entry.is_dir() || !archive_image_name(&name) {
            continue;
        }
        let size = entry.size();
        drop(entry);
        let mut entry = archive
            .by_index(index)
            .with_context(|| format!("无法读取 {name}；不支持加密压缩包"))?;
        sink.push(&name, size, &mut entry)?;
    }
    Ok(())
}

fn extract_7z(path: &Path, sink: &mut Extractor<'_>) -> Result<()> {
    let mut archive = sevenz_rust2::ArchiveReader::open(path, sevenz_rust2::Password::empty())
        .context("无法读取 7Z；请确认文件完整且未加密")?;
    archive
        .for_each_entries(|entry, reader| {
            check_cancel(sink.cancel).map_err(|e| std::io::Error::other(e.to_string()))?;
            if !entry.is_directory() && archive_image_name(entry.name()) {
                sink.push(entry.name(), entry.size(), reader)
                    .map_err(|e| std::io::Error::other(format!("{e:#}")))?;
            } else {
                // Solid archives require consuming entries even when they are ignored.
                io::copy(reader, &mut io::sink())?;
            }
            Ok(true)
        })
        .context("7Z 解压失败")?;
    Ok(())
}

fn extract_rar(path: &Path, sink: &mut Extractor<'_>) -> Result<()> {
    let _guard = loop {
        check_cancel(sink.cancel)?;
        match RAR_LOCK.try_lock() {
            Ok(guard) => break guard,
            Err(TryLockError::WouldBlock) => {
                std::thread::sleep(std::time::Duration::from_millis(20))
            }
            Err(TryLockError::Poisoned(_)) => bail!("RAR 解压器状态异常，请重启程序后再试"),
        }
    };
    let mut archive = unrar::Archive::new(path)
        .open_for_processing()
        .context("无法读取 RAR；请确认文件完整且未加密")?;
    while let Some(entry) = archive.read_header().context("RAR 读取失败")? {
        check_cancel(sink.cancel)?;
        let name = entry.entry().filename.to_string_lossy().to_string();
        let size = entry.entry().unpacked_size;
        if !entry.entry().is_directory() && archive_image_name(&name) {
            ensure!(
                size <= MAX_PAGE_BYTES && sink.total.saturating_add(size) <= MAX_TOTAL_BYTES,
                "RAR 图片过大：{name}"
            );
            let (bytes, rest) = entry
                .read()
                .context("RAR 解压失败；不支持加密或缺失分卷的压缩包")?;
            sink.push(&name, size, &mut bytes.as_slice())?;
            archive = rest;
        } else {
            archive = entry.skip()?;
        }
    }
    Ok(())
}

fn extract_tar(reader: impl Read, sink: &mut Extractor<'_>) -> Result<()> {
    for entry in tar::Archive::new(reader)
        .entries()
        .context("无法读取 TAR 压缩包")?
    {
        check_cancel(sink.cancel)?;
        let mut entry = entry?;
        let name = entry.path()?.to_string_lossy().to_string();
        if entry.header().entry_type().is_file() && archive_image_name(&name) {
            sink.push(&name, entry.size(), &mut entry)?;
        }
    }
    Ok(())
}
