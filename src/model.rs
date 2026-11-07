use std::path::{Path, PathBuf};

pub const OUTPUT_FOLDER: &str = "Telysta Books";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum BookFormat {
    #[default]
    Epub,
    Pdf,
    Cbz,
}

impl BookFormat {
    pub const ALL: [Self; 3] = [Self::Epub, Self::Pdf, Self::Cbz];

    pub fn extension(self) -> &'static str {
        match self {
            Self::Epub => "epub",
            Self::Pdf => "pdf",
            Self::Cbz => "cbz",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Epub => "EPUB",
            Self::Pdf => "PDF",
            Self::Cbz => "CBZ",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Epub => "电子书 · 默认推荐",
            Self::Pdf => "通用文档 · 随处阅读",
            Self::Cbz => "漫画归档 · 保留原图",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum Direction {
    #[default]
    Ltr,
    Rtl,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BookSource {
    Path(PathBuf),
    Images(Vec<PathBuf>),
}

impl BookSource {
    pub fn path(&self) -> &Path {
        match self {
            Self::Path(path) => path,
            Self::Images(paths) => paths
                .first()
                .map(PathBuf::as_path)
                .unwrap_or(Path::new(".")),
        }
    }

    pub fn title(&self) -> String {
        let path = match self {
            Self::Images(paths) if paths.len() > 1 => paths[0].parent().unwrap_or(&paths[0]),
            _ => self.path(),
        };
        let name = if path.is_dir() {
            path.file_name()
        } else {
            path.file_stem()
        };
        let name = name.unwrap_or_default().to_string_lossy().to_string();
        name.strip_suffix(".tar").unwrap_or(&name).to_string()
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Images(_) => "图片合集",
            Self::Path(path) if path.is_dir() => "文件夹",
            _ => "压缩包",
        }
    }

    pub fn default_output_parent(&self) -> PathBuf {
        self.path().parent().unwrap_or(Path::new(".")).to_path_buf()
    }
}

#[derive(Clone, Debug)]
pub struct ConversionOptions {
    pub format: BookFormat,
    /// Parent directory: a new `Telysta Books` subdirectory is created here.
    pub output_parent: PathBuf,
    pub title: String,
    pub author: String,
    pub direction: Direction,
    pub jpeg_quality: u8,
}

impl ConversionOptions {
    pub fn for_source(source: &BookSource) -> Self {
        Self {
            format: BookFormat::Epub,
            output_parent: source.default_output_parent(),
            title: source.title(),
            author: String::new(),
            direction: Direction::Ltr,
            jpeg_quality: 90,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Progress {
    pub fraction: f32,
    pub message: String,
    pub pages_done: usize,
    pub pages_total: usize,
}

#[derive(Clone, Debug)]
pub struct ConversionResult {
    pub path: PathBuf,
    pub pages: usize,
    pub bytes: u64,
}
