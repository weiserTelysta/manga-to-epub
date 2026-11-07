use eframe::egui;

mod components;
mod theme;
mod view;

#[cfg(test)]
mod tests;

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
};
use telysta::{
    BatchEvent, BookFormat, BookSource, ConversionJob, ConversionOptions, ConversionResult,
    Direction, Progress, convert_batch, discover, model::OUTPUT_FOLDER, preview,
};
use theme::Appearance;

#[derive(Clone)]
enum Status {
    Pending,
    Running(Progress),
    Done(ConversionResult),
    Failed(String),
    Cancelled,
}

struct Task {
    id: u64,
    source: BookSource,
    title: String,
    status: Status,
    thumbnail: Option<egui::TextureHandle>,
    preview_requested: bool,
}

enum Event {
    Imported(Result<Vec<BookSource>, String>),
    PickedOutput(Option<PathBuf>),
    Thumbnail(u64, Option<image::RgbaImage>),
    Progress(u64, Progress),
    Finished(u64, Result<ConversionResult, String>),
    BatchDone,
}

pub struct TelystaApp {
    appearance: Appearance,
    preview_open: bool,
    error_details: Option<String>,
    tasks: Vec<Task>,
    selected: Option<u64>,
    next_id: u64,
    format: BookFormat,
    author: String,
    quality: u8,
    parallelism: usize,
    direction: Direction,
    output_parent: Option<PathBuf>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    busy: bool,
    importing: bool,
    choosing_output: bool,
    message: String,
    help: bool,
    batch_ids: Vec<u64>,
    screenshot_started: std::time::Instant,
    screenshot_requested: bool,
}

impl TelystaApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        view::install_fonts(&cc.egui_ctx);
        let saved_theme = std::env::var("TELYSTA_THEME").ok().or_else(|| {
            cc.storage
                .and_then(|storage| storage.get_string("telysta.appearance"))
        });
        let appearance = match saved_theme.as_deref() {
            Some("rose-pine") => Appearance::RosePine,
            Some("night" | "dark") => Appearance::Night,
            _ => Appearance::Campus,
        };
        appearance.apply(&cc.egui_ctx);
        let mut app = Self::with_appearance(appearance);
        let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
        if !paths.is_empty() {
            app.import(paths, &cc.egui_ctx);
        }
        app
    }

    fn with_appearance(appearance: Appearance) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            appearance,
            preview_open: false,
            error_details: None,
            tasks: Vec::new(),
            selected: None,
            next_id: 0,
            format: BookFormat::Epub,
            author: String::new(),
            quality: 90,
            parallelism: telysta::batch::DEFAULT_JOBS,
            direction: Direction::Ltr,
            output_parent: None,
            tx,
            rx,
            cancel: Arc::new(AtomicBool::new(false)),
            busy: false,
            importing: false,
            choosing_output: false,
            message: String::new(),
            help: false,
            batch_ids: Vec::new(),
            screenshot_started: std::time::Instant::now(),
            screenshot_requested: false,
        }
    }

    fn import(&mut self, paths: Vec<PathBuf>, ctx: &egui::Context) {
        if paths.is_empty() || self.importing {
            return;
        }
        self.importing = true;
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        thread::spawn(move || {
            let result = discover(paths).map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::Imported(result));
            ctx.request_repaint();
        });
    }

    fn pick(&mut self, folder: bool, ctx: &egui::Context) {
        if self.importing {
            return;
        }
        self.importing = true;
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        thread::spawn(move || {
            let dialog = rfd::FileDialog::new().set_title(if folder {
                "选择图片或压缩包所在的文件夹"
            } else {
                "选择图片或压缩包"
            });
            let paths = if folder {
                dialog.pick_folder().map(|p| vec![p])
            } else {
                dialog
                    .add_filter(
                        "图片与压缩包",
                        &[
                            "zip", "cbz", "7z", "rar", "cbr", "tar", "gz", "tgz", "jpg", "jpeg",
                            "png", "webp", "gif", "bmp", "tif", "tiff",
                        ],
                    )
                    .pick_files()
            };
            let result = paths
                .map(discover)
                .transpose()
                .map(|v| v.unwrap_or_default())
                .map_err(|e| format!("{e:#}"));
            let _ = tx.send(Event::Imported(result));
            ctx.request_repaint();
        });
    }

    fn receive(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Imported(result) => {
                    self.importing = false;
                    match result {
                        Ok(sources) => {
                            for source in sources {
                                if self.tasks.iter().any(|t| t.source == source) {
                                    continue;
                                }
                                let id = self.next_id;
                                self.next_id += 1;
                                let title = source.title();
                                self.tasks.push(Task {
                                    id,
                                    source,
                                    title,
                                    status: Status::Pending,
                                    thumbnail: None,
                                    preview_requested: false,
                                });
                                if self.selected.is_none() {
                                    self.selected = Some(id);
                                }
                            }
                            self.message.clear();
                        }
                        Err(error) => self.message = error,
                    }
                }
                Event::PickedOutput(path) => {
                    self.choosing_output = false;
                    if path.is_some() {
                        self.output_parent = path;
                    }
                }
                Event::Thumbnail(id, image) => {
                    if let (Some(task), Some(image)) =
                        (self.tasks.iter_mut().find(|t| t.id == id), image)
                    {
                        let size = [image.width() as usize, image.height() as usize];
                        task.thumbnail = Some(ctx.load_texture(
                            format!("cover-{id}"),
                            egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                }
                Event::Progress(id, progress) => {
                    if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                        task.status = Status::Running(progress);
                    }
                }
                Event::Finished(id, result) => {
                    if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                        task.status = match result {
                            Ok(result) => Status::Done(result),
                            Err(_) if self.cancel.load(Ordering::Relaxed) => Status::Cancelled,
                            Err(error) => Status::Failed(error),
                        };
                    }
                }
                Event::BatchDone => {
                    self.busy = false;
                    for task in &mut self.tasks {
                        if self.batch_ids.contains(&task.id)
                            && matches!(task.status, Status::Pending | Status::Running(_))
                        {
                            task.status = Status::Cancelled;
                        }
                    }
                    let done = self
                        .tasks
                        .iter()
                        .filter(|t| {
                            self.batch_ids.contains(&t.id) && matches!(t.status, Status::Done(_))
                        })
                        .count();
                    let failed = self
                        .tasks
                        .iter()
                        .filter(|t| {
                            self.batch_ids.contains(&t.id) && matches!(t.status, Status::Failed(_))
                        })
                        .count();
                    self.message = format!(
                        "制作结束 · {done} 本完成 · {failed} 本失败{}　",
                        if self.cancel.load(Ordering::Relaxed) {
                            " · 已停止剩余任务"
                        } else {
                            ""
                        }
                    );
                }
            }
        }
        if let Some(task) = self
            .tasks
            .iter_mut()
            .find(|t| Some(t.id) == self.selected && !t.preview_requested)
        {
            task.preview_requested = true;
            let (id, source, tx, ctx) =
                (task.id, task.source.clone(), self.tx.clone(), ctx.clone());
            thread::spawn(move || {
                let image = preview(&source).ok().flatten();
                let _ = tx.send(Event::Thumbnail(id, image));
                ctx.request_repaint();
            });
        }
    }

    fn start(&mut self, ctx: &egui::Context) {
        let jobs: Vec<_> = self
            .tasks
            .iter()
            .filter(|t| !matches!(t.status, Status::Done(_)))
            .map(|t| {
                let mut options = ConversionOptions::for_source(&t.source);
                options.title = t.title.clone();
                options.author = self.author.clone();
                options.format = self.format;
                options.direction = self.direction;
                options.jpeg_quality = self.quality;
                if let Some(parent) = &self.output_parent {
                    options.output_parent = parent.clone();
                }
                (t.id, t.source.clone(), options)
            })
            .collect();
        if jobs.is_empty() || self.busy {
            return;
        }
        self.batch_ids = jobs.iter().map(|(id, _, _)| *id).collect();
        for task in &mut self.tasks {
            if self.batch_ids.contains(&task.id) {
                task.status = Status::Pending;
            }
        }
        self.busy = true;
        self.message.clear();
        self.cancel = Arc::new(AtomicBool::new(false));
        let (tx, cancel, ctx) = (self.tx.clone(), self.cancel.clone(), ctx.clone());
        let parallelism = self.parallelism;
        thread::spawn(move || {
            let ids = jobs.iter().map(|(id, _, _)| *id).collect::<Vec<_>>();
            let jobs = jobs
                .into_iter()
                .map(|(_, source, options)| ConversionJob { source, options })
                .collect();
            let _ = convert_batch(jobs, parallelism, &cancel, |event| {
                match event {
                    BatchEvent::Progress { index, progress } => {
                        let _ = tx.send(Event::Progress(ids[index], progress));
                    }
                    BatchEvent::Finished { index, result } => {
                        let _ = tx.send(Event::Finished(
                            ids[index],
                            result.map_err(|error| format!("{error:#}")),
                        ));
                    }
                }
                ctx.request_repaint();
            });
            let _ = tx.send(Event::BatchDone);
            ctx.request_repaint();
        });
    }

    fn output_path(&self) -> String {
        if self.output_parent.is_none()
            && let Some(first) = self.tasks.first()
            && self.tasks.iter().any(|task| {
                task.source.default_output_parent() != first.source.default_output_parent()
            })
        {
            return "各源文件所在目录 / Telysta Books".into();
        }
        self.output_parent
            .clone()
            .or_else(|| self.tasks.first().map(|t| t.source.default_output_parent()))
            .map(|p| p.join(OUTPUT_FOLDER).display().to_string())
            .unwrap_or_else(|| "源文件所在目录 / Telysta Books".into())
    }

    fn set_format(&mut self, format: BookFormat) {
        if self.format == format {
            return;
        }
        self.format = format;
        self.message.clear();
        for task in &mut self.tasks {
            if matches!(task.status, Status::Done(_)) {
                task.status = Status::Pending;
            }
        }
    }

    fn open_output(&mut self) {
        let parent = self.tasks.iter().rev().find_map(|task| {
            if let Status::Done(result) = &task.status {
                result.path.parent().map(|p| p.to_owned())
            } else {
                None
            }
        });
        if let Some(parent) = parent
            && let Err(error) = open::that_detached(parent)
        {
            self.message = format!("无法打开输出文件夹：{error}");
        }
    }
}

impl eframe::App for TelystaApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if std::env::var_os("TELYSTA_SCREENSHOT").is_none() {
            storage.set_string(
                "telysta.appearance",
                match self.appearance {
                    Appearance::Campus => "campus",
                    Appearance::Night => "night",
                    Appearance::RosePine => "rose-pine",
                }
                .into(),
            );
        }
    }

    fn persist_egui_memory(&self) -> bool {
        std::env::var_os("TELYSTA_SCREENSHOT").is_none()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive(ctx);
        let dropped = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect::<Vec<_>>()
        });
        if !dropped.is_empty() && !self.busy {
            self.import(dropped, ctx);
        }
        if self.busy && ctx.input(|i| i.viewport().close_requested()) {
            self.cancel.store(true, Ordering::Relaxed);
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.message = "正在停止制作，清理完成后即可关闭窗口。".into();
        }
        self.shortcuts(ctx);
        self.render(ctx);
        self.capture(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
