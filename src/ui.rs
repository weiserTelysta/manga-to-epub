use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Frame, Layout, Margin, Pos2, Rect,
    RichText, Sense, Stroke, StrokeKind, Vec2,
};
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
    BookFormat, BookSource, ConversionOptions, ConversionResult, Direction, Progress, convert,
    discover, model::OUTPUT_FOLDER, preview,
};

const BASE: Color32 = Color32::from_rgb(0x19, 0x17, 0x24);
const SURFACE: Color32 = Color32::from_rgb(0x1f, 0x1d, 0x2e);
const OVERLAY: Color32 = Color32::from_rgb(0x26, 0x23, 0x3a);
const BORDER: Color32 = Color32::from_rgb(0x40, 0x3d, 0x52);
const MUTED: Color32 = Color32::from_rgb(0x6e, 0x6a, 0x86);
const SUBTLE: Color32 = Color32::from_rgb(0x90, 0x8c, 0xaa);
const TEXT: Color32 = Color32::from_rgb(0xe0, 0xde, 0xf4);
const ROSE: Color32 = Color32::from_rgb(0xeb, 0xbc, 0xba);
const IRIS: Color32 = Color32::from_rgb(0xc4, 0xa7, 0xe7);
const FOAM: Color32 = Color32::from_rgb(0x9c, 0xcf, 0xd8);
const GOLD: Color32 = Color32::from_rgb(0xf6, 0xc1, 0x77);
const LOVE: Color32 = Color32::from_rgb(0xeb, 0x6f, 0x92);

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
    tasks: Vec<Task>,
    selected: Option<u64>,
    next_id: u64,
    format: BookFormat,
    author: String,
    quality: u8,
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
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "noto-sc".into(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/NotoSansSC.ttf")).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "noto-sc".into());
        cc.egui_ctx.set_fonts(fonts);
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = BASE;
        style.visuals.window_fill = SURFACE;
        style.visuals.extreme_bg_color = BASE;
        style.visuals.faint_bg_color = SURFACE;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.selection.bg_fill = BORDER;
        style.visuals.selection.stroke = Stroke::new(1.0_f32, ROSE);
        style.visuals.widgets.inactive.bg_fill = OVERLAY;
        style.visuals.widgets.inactive.weak_bg_fill = OVERLAY;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
        style.visuals.widgets.hovered.bg_fill = BORDER;
        style.visuals.widgets.hovered.weak_bg_fill = BORDER;
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, IRIS);
        style.visuals.widgets.active.bg_fill = BORDER;
        style.visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ROSE);
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
        ] {
            widget.corner_radius = CornerRadius::same(7);
            widget.fg_stroke = Stroke::new(1.0_f32, TEXT);
        }
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(14.0, 9.0);
        style.spacing.interact_size.y = 32.0;
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, FontId::proportional(12.0));
        cc.egui_ctx.set_style(style);
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            tasks: Vec::new(),
            selected: None,
            next_id: 0,
            format: BookFormat::Epub,
            author: String::new(),
            quality: 90,
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
        };
        let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
        if !paths.is_empty() {
            app.import(paths, &cc.egui_ctx);
        }
        app
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
        thread::spawn(move || {
            for (id, source, options) in jobs {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    convert(&source, &options, &cancel, |p| {
                        let _ = tx.send(Event::Progress(id, p));
                        ctx.request_repaint();
                    })
                    .map_err(|e| format!("{e:#}"))
                }))
                .unwrap_or_else(|_| Err("转换遇到内部错误，请检查该文件".into()));
                let _ = tx.send(Event::Finished(id, result));
                ctx.request_repaint();
            }
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

    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("brand")
            .exact_height(78.0)
            .frame(
                Frame::new()
                    .fill(SURFACE)
                    .inner_margin(Margin::symmetric(28, 17)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::hover());
                    ui.painter().rect_filled(rect, 11, OVERLAY);
                    draw_book(ui.painter(), rect.shrink(10.0), ROSE);
                    ui.add_space(5.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("T E L Y S T A")
                                .size(19.0)
                                .color(TEXT)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("I M A G E   T O   E B O O K")
                                .size(9.0)
                                .color(SUBTLE),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        pill(ui, "Rosé Pine", IRIS);
                        ui.add_space(10.0);
                        ui.label(RichText::new("●  纯本地制作").size(12.0).color(FOAM));
                    });
                });
            });
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(178.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(SURFACE)
                    .inner_margin(Margin::symmetric(18, 28)),
            )
            .show(ctx, |ui| {
                ui.label(RichText::new("W O R K S P A C E").size(10.0).color(MUTED));
                ui.add_space(16.0);
                Frame::new()
                    .fill(OVERLAY)
                    .corner_radius(8)
                    .inner_margin(Margin::symmetric(12, 12))
                    .show(ui, |ui| {
                        ui.set_width(118.0);
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(17.0), Sense::hover());
                            draw_book(ui.painter(), r, ROSE);
                            ui.label(RichText::new("制作书籍").color(ROSE).size(14.0));
                        });
                    });
                ui.add_space(8.0);
                if ui
                    .add(
                        egui::Button::new(RichText::new("  ?   使用指南").color(SUBTLE))
                            .frame(false)
                            .min_size(Vec2::new(140.0, 40.0)),
                    )
                    .clicked()
                {
                    self.help = true;
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.label(
                        RichText::new("v0.1.0  /  MADE WITH RUST")
                            .size(9.0)
                            .color(MUTED),
                    );
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new("为每一页，\n留一处归所。")
                            .size(13.0)
                            .line_height(Some(24.0))
                            .color(SUBTLE),
                    );
                    ui.add_space(12.0);
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                    let c = r.center();
                    ui.painter().line_segment(
                        [c - Vec2::new(7.0, 0.0), c + Vec2::new(7.0, 0.0)],
                        Stroke::new(1.2_f32, ROSE),
                    );
                    ui.painter().line_segment(
                        [c - Vec2::new(0.0, 9.0), c + Vec2::new(0.0, 9.0)],
                        Stroke::new(1.2_f32, ROSE),
                    );
                });
            });
    }

    fn preview_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("preview")
            .exact_width(278.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(BASE)
                    .inner_margin(Margin::symmetric(26, 30)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("书籍预览").size(15.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("PREVIEW").size(9.0).color(MUTED));
                    });
                });
                ui.add_space(25.0);
                let task = self.tasks.iter_mut().find(|t| Some(t.id) == self.selected);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(224.0, 304.0), Sense::hover());
                let cover_rect =
                    Rect::from_min_size(rect.min + Vec2::new(10.0, 0.0), Vec2::new(204.0, 286.0));
                ui.painter().rect_filled(
                    cover_rect.translate(Vec2::new(5.0, 8.0)),
                    4,
                    Color32::from_black_alpha(65),
                );
                if let Some(texture) = task.as_ref().and_then(|t| t.thumbnail.as_ref()) {
                    ui.painter().rect_filled(cover_rect, 4, SURFACE);
                    let size = texture.size_vec2();
                    let fit = (cover_rect.width() / size.x).min(cover_rect.height() / size.y);
                    let r = Rect::from_center_size(cover_rect.center(), size * fit);
                    ui.painter().image(
                        texture.id(),
                        r,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    ui.painter().rect_stroke(
                        cover_rect,
                        4,
                        Stroke::new(1.0_f32, BORDER),
                        StrokeKind::Inside,
                    );
                } else {
                    draw_cover(ui.painter(), cover_rect);
                }
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(
                            task.as_ref()
                                .map(|t| t.title.as_str())
                                .unwrap_or("你的下一本珍藏"),
                        )
                        .size(17.0)
                        .color(TEXT),
                    );
                    ui.add_space(1.0);
                    ui.label(
                        RichText::new(if task.is_some() {
                            "第一张图片将成为书籍封面"
                        } else {
                            "从一张图片，开始一段阅读"
                        })
                        .size(11.0)
                        .color(SUBTLE),
                    );
                });
                ui.add_space(24.0);
                ui.separator();
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("输出格式").size(12.0).color(SUBTLE));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        pill(ui, self.format.label(), ROSE);
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("排版方式").size(12.0).color(SUBTLE));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("一图一页 · 保留比例").size(12.0));
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("阅读顺序").size(12.0).color(SUBTLE));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(if self.direction == Direction::Ltr {
                                "从左至右 →"
                            } else {
                                "← 从右至左"
                            })
                            .size(12.0),
                        );
                    });
                });
                ui.add_space(20.0);
                if let Some(task) = task {
                    ui.label(RichText::new("书名").size(11.0).color(SUBTLE));
                    ui.add_enabled(
                        !self.busy,
                        egui::TextEdit::singleline(&mut task.title)
                            .desired_width(224.0)
                            .margin(Vec2::splat(8.0)),
                    );
                    if let Status::Done(result) = &task.status {
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(format!(
                                "{} 页 · {:.1} MiB",
                                result.pages,
                                result.bytes as f64 / 1_048_576.0
                            ))
                            .color(FOAM),
                        );
                        if ui.button("打开这本书 ↗").clicked()
                            && let Err(e) = open::that_detached(&result.path)
                        {
                            self.message = format!("无法打开书籍：{e}");
                        }
                    }
                }
                if ui.available_height() > 110.0 {
                    ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                        Frame::new()
                            .fill(SURFACE)
                            .corner_radius(8)
                            .inner_margin(14)
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new("小小的阅读仪式").size(12.0).color(IRIS),
                                    );
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new(
                                            "图片按文件名自然排序。\n第 2 页，总会在第 10 页之前。",
                                        )
                                        .size(11.0)
                                        .color(SUBTLE)
                                        .line_height(Some(20.0)),
                                    );
                                });
                            });
                    });
                }
            });
    }

    fn footer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("action")
            .exact_height(84.0)
            .frame(
                Frame::new()
                    .fill(SURFACE)
                    .inner_margin(Margin::symmetric(28, 15)),
            )
            .show(ctx, |ui| {
                let remaining = self
                    .tasks
                    .iter()
                    .filter(|t| !matches!(t.status, Status::Done(_)))
                    .count();
                ui.horizontal_centered(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 255.0).max(220.0));
                        if self.busy {
                            let mut fraction = 0.0;
                            let mut message = "等待制作…".to_owned();
                            for task in self.tasks.iter().filter(|t| self.batch_ids.contains(&t.id))
                            {
                                match &task.status {
                                    Status::Done(_) | Status::Failed(_) | Status::Cancelled => {
                                        fraction += 1.0
                                    }
                                    Status::Running(p) => {
                                        fraction += p.fraction;
                                        message.clone_from(&p.message);
                                    }
                                    _ => {}
                                }
                            }
                            ui.label(
                                RichText::new(if self.cancel.load(Ordering::Relaxed) {
                                    "正在停止并清理临时文件…"
                                } else {
                                    &message
                                })
                                .size(12.0)
                                .color(ROSE),
                            );
                            ui.add(
                                egui::ProgressBar::new(
                                    fraction / self.batch_ids.len().max(1) as f32,
                                )
                                .desired_height(5.0)
                                .fill(ROSE)
                                .corner_radius(3),
                            );
                        } else {
                            ui.label(
                                RichText::new(if self.tasks.is_empty() {
                                    "每一本好书，都从这里开始。".into()
                                } else if remaining == 0 {
                                    "制作完成，去翻开你的新书吧。".into()
                                } else {
                                    format!("已准备 {remaining} 本书籍")
                                })
                                .size(13.0)
                                .color(TEXT),
                            );
                            ui.label(
                                RichText::new("自动新建输出文件夹 · 同名书籍自动编号")
                                    .size(11.0)
                                    .color(MUTED),
                            );
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if self.busy {
                            if ui
                                .add_enabled(
                                    !self.cancel.load(Ordering::Relaxed),
                                    egui::Button::new("停止制作").min_size(Vec2::new(180.0, 44.0)),
                                )
                                .clicked()
                            {
                                self.cancel.store(true, Ordering::Relaxed);
                            }
                        } else {
                            let enabled = remaining > 0
                                && !self.importing
                                && !self.choosing_output
                                && self
                                    .tasks
                                    .iter()
                                    .filter(|t| !matches!(t.status, Status::Done(_)))
                                    .all(|t| !t.title.trim().is_empty());
                            let text = if remaining > 0 {
                                format!("开始制作   ·   {remaining} 本  →")
                            } else {
                                "开始制作  →".into()
                            };
                            if ui
                                .add_enabled(
                                    enabled,
                                    egui::Button::new(
                                        RichText::new(text).size(14.0).strong().color(BASE),
                                    )
                                    .fill(ROSE)
                                    .corner_radius(8)
                                    .min_size(Vec2::new(205.0, 44.0)),
                                )
                                .clicked()
                            {
                                self.start(ctx);
                            }
                        }
                    });
                });
            });
    }

    fn drop_zone(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        let fill = if hovering { OVERLAY } else { SURFACE };
        let stroke = if hovering { ROSE } else { BORDER };
        let frame = Frame::new()
            .fill(fill)
            .corner_radius(12)
            .inner_margin(Margin::symmetric(20, 14));
        let response = frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 5.0;
                let (rect, _) = ui.allocate_exact_size(Vec2::new(38.0, 36.0), Sense::hover());
                let p = ui.painter();
                p.rect_stroke(
                    rect.shrink(5.0).translate(Vec2::new(5.0, -3.0)),
                    3,
                    Stroke::new(1.2_f32, MUTED),
                    StrokeKind::Inside,
                );
                p.rect_filled(rect.shrink(5.0), 3, OVERLAY);
                p.rect_stroke(
                    rect.shrink(5.0),
                    3,
                    Stroke::new(1.3_f32, IRIS),
                    StrokeKind::Inside,
                );
                let r = rect.shrink(11.0);
                p.line_segment([r.left_bottom(), r.center()], Stroke::new(1.2_f32, IRIS));
                p.line_segment([r.center(), r.right_bottom()], Stroke::new(1.2_f32, IRIS));
                p.circle_filled(r.left_top() + Vec2::new(4.0, 3.0), 2.0, ROSE);
                ui.add_space(6.0);
                ui.label(
                    RichText::new(if self.importing {
                        "正在整理所选文件…"
                    } else {
                        "拖入图片、文件夹或压缩包"
                    })
                    .size(16.0)
                    .color(if hovering { ROSE } else { TEXT }),
                );
                ui.label(
                    RichText::new("ZIP / 7Z / RAR / TAR  ·  JPG / PNG / WEBP / 更多")
                        .size(10.0)
                        .color(SUBTLE),
                );
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    let total = 230.0;
                    ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                    if ui
                        .add_enabled(
                            !self.importing,
                            egui::Button::new(RichText::new("＋  选择文件").color(ROSE))
                                .min_size(Vec2::new(110.0, 33.0)),
                        )
                        .clicked()
                    {
                        self.pick(false, ctx);
                    }
                    if ui
                        .add_enabled(
                            !self.importing,
                            egui::Button::new("选择文件夹").min_size(Vec2::new(110.0, 33.0)),
                        )
                        .clicked()
                    {
                        self.pick(true, ctx);
                    }
                });
            });
        });
        ui.painter().rect_stroke(
            response.response.rect,
            12,
            Stroke::new(1.0_f32, stroke),
            StrokeKind::Inside,
        );
    }

    fn queue(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("待装订的书籍").size(14.0).strong());
            pill(ui, &format!("{:02}", self.tasks.len()), IRIS);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(
                        !self.busy && !self.tasks.is_empty(),
                        egui::Button::new(RichText::new("清空").size(11.0).color(SUBTLE))
                            .frame(false),
                    )
                    .clicked()
                {
                    self.tasks.clear();
                    self.selected = None;
                    self.message.clear();
                }
            });
        });
        if self.tasks.is_empty() {
            Frame::new()
                .fill(SURFACE)
                .corner_radius(8)
                .inner_margin(18)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("还没有书籍，添加一份值得珍藏的图片吧。")
                            .size(12.0)
                            .color(MUTED),
                    );
                });
            return;
        }
        let mut remove = None;
        egui::ScrollArea::vertical()
            .id_salt("queue-scroll")
            .max_height(190.0)
            .show(ui, |ui| {
                for task in &self.tasks {
                    let selected = self.selected == Some(task.id);
                    let response = Frame::new()
                        .fill(if selected { OVERLAY } else { SURFACE })
                        .corner_radius(8)
                        .stroke(Stroke::new(
                            1.0_f32,
                            if selected { BORDER } else { SURFACE },
                        ))
                        .inner_margin(Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (r, _) =
                                    ui.allocate_exact_size(Vec2::new(30.0, 36.0), Sense::hover());
                                ui.painter().rect_filled(
                                    r,
                                    5,
                                    if selected {
                                        Color32::from_rgb(0x34, 0x2a, 0x40)
                                    } else {
                                        OVERLAY
                                    },
                                );
                                draw_book(
                                    ui.painter(),
                                    r.shrink(7.0),
                                    if selected { IRIS } else { SUBTLE },
                                );
                                let available = (ui.available_width() - 102.0).max(140.0);
                                ui.vertical(|ui| {
                                    ui.set_width(available);
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&task.title).size(13.0).color(TEXT),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(task.source.path().display().to_string());
                                    let (detail, color) = match &task.status {
                                        Status::Pending => {
                                            (format!("{} · 等待制作", task.source.kind()), SUBTLE)
                                        }
                                        Status::Running(p) => (p.message.clone(), ROSE),
                                        Status::Done(r) => (
                                            format!(
                                                "{} 页 · {:.1} MiB · 制作完成",
                                                r.pages,
                                                r.bytes as f64 / 1_048_576.0
                                            ),
                                            FOAM,
                                        ),
                                        Status::Failed(e) => (e.clone(), LOVE),
                                        Status::Cancelled => ("已取消 · 可重新制作".into(), GOLD),
                                    };
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&detail).size(10.0).color(color),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(detail);
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui
                                        .add_enabled(
                                            !self.busy,
                                            egui::Button::new(
                                                RichText::new("×").size(17.0).color(MUTED),
                                            )
                                            .frame(false)
                                            .min_size(Vec2::new(22.0, 28.0)),
                                        )
                                        .clicked()
                                    {
                                        remove = Some(task.id);
                                    }
                                    match task.status {
                                        Status::Done(_) => {
                                            ui.label(RichText::new("✓").color(FOAM));
                                        }
                                        Status::Running(_) => {
                                            ui.spinner();
                                        }
                                        Status::Failed(_) => {
                                            ui.label(RichText::new("!").color(LOVE));
                                        }
                                        _ => {
                                            ui.label(
                                                RichText::new("待制作").size(10.0).color(MUTED),
                                            );
                                        }
                                    }
                                });
                            });
                        });
                    // Only the row background is selectable; child controls keep their clicks.
                    if ui
                        .interact(
                            response.response.rect,
                            ui.id().with(("row", task.id)),
                            Sense::click(),
                        )
                        .clicked()
                    {
                        self.selected = Some(task.id);
                    }
                }
            });
        if let Some(id) = remove {
            self.tasks.retain(|t| t.id != id);
            if self.selected == Some(id) {
                self.selected = self.tasks.first().map(|t| t.id);
            }
        }
    }

    fn settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label(RichText::new("装订选项").size(14.0).strong());
        ui.add_enabled_ui(!self.busy, |ui| {
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 20.0) / 3.0;
                for format in BookFormat::ALL {
                    let active = self.format == format;
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, 74.0), Sense::click());
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::RadioButton,
                            ui.is_enabled(),
                            active,
                            format.label(),
                        )
                    });
                    let fill = if active {
                        Color32::from_rgb(0x30, 0x26, 0x36)
                    } else if response.hovered() {
                        OVERLAY
                    } else {
                        SURFACE
                    };
                    ui.painter().rect_filled(rect, 8, fill);
                    ui.painter().rect_stroke(
                        rect,
                        8,
                        Stroke::new(1.0_f32, if active { ROSE } else { BORDER }),
                        StrokeKind::Inside,
                    );
                    ui.painter().text(
                        rect.min + Vec2::new(13.0, 16.0),
                        Align2::LEFT_TOP,
                        format.label(),
                        FontId::proportional(16.0),
                        if active { ROSE } else { TEXT },
                    );
                    ui.painter().text(
                        rect.min + Vec2::new(13.0, 47.0),
                        Align2::LEFT_TOP,
                        format.description(),
                        FontId::proportional(10.0),
                        SUBTLE,
                    );
                    let dot = Pos2::new(rect.right() - 17.0, rect.top() + 24.0);
                    if active {
                        ui.painter().circle_filled(dot, 4.0, ROSE);
                    } else {
                        ui.painter()
                            .circle_stroke(dot, 4.0, Stroke::new(1.0_f32, MUTED));
                    }
                    if response.clicked() {
                        self.format = format;
                    }
                }
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("作者").size(12.0).color(SUBTLE));
                ui.add(
                    egui::TextEdit::singleline(&mut self.author)
                        .hint_text("可选")
                        .desired_width((ui.available_width() - 240.0).max(90.0))
                        .margin(Vec2::new(8.0, 7.0)),
                );
                ui.add_space(8.0);
                ui.label(RichText::new("翻页方向").size(12.0).color(SUBTLE));
                egui::ComboBox::from_id_salt("direction")
                    .selected_text(if self.direction == Direction::Ltr {
                        "从左至右"
                    } else {
                        "从右至左"
                    })
                    .width(98.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.direction, Direction::Ltr, "从左至右");
                        ui.selectable_value(&mut self.direction, Direction::Rtl, "从右至左");
                    });
            });
            if self.format != BookFormat::Cbz {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("转码质量").size(11.0).color(SUBTLE));
                    ui.add(
                        egui::Slider::new(&mut self.quality, 60..=100)
                            .suffix("%")
                            .show_value(true),
                    );
                    ui.label(
                        RichText::new(if self.format == BookFormat::Epub {
                            "JPEG / PNG 原图直接保留"
                        } else {
                            "图片按原始比例嵌入 PDF"
                        })
                        .size(10.0)
                        .color(MUTED),
                    );
                });
            }
        });
        ui.add_space(6.0);
        Frame::new()
            .fill(SURFACE)
            .corner_radius(8)
            .inner_margin(Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 120.0).max(150.0));
                        ui.label(RichText::new("输出位置").size(12.0).color(TEXT));
                        let path = self.output_path();
                        ui.add(
                            egui::Label::new(RichText::new(&path).size(10.0).color(SUBTLE))
                                .truncate(),
                        )
                        .on_hover_text(path);
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                !self.busy && !self.choosing_output,
                                egui::Button::new(RichText::new("更改位置").size(11.0)),
                            )
                            .clicked()
                        {
                            self.choosing_output = true;
                            let (tx, ctx) = (self.tx.clone(), ctx.clone());
                            thread::spawn(move || {
                                let result = rfd::FileDialog::new()
                                    .set_title("选择输出父目录（将自动新建 Telysta Books）")
                                    .pick_folder();
                                let _ = tx.send(Event::PickedOutput(result));
                                ctx.request_repaint();
                            });
                        }
                    });
                });
                if self.output_parent.is_some()
                    && !self.busy
                    && ui.small_button("恢复到各源文件所在目录").clicked()
                {
                    self.output_parent = None;
                }
            });
    }
}

impl eframe::App for TelystaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive(ctx);
        let dropped = ctx.input(|i| {
            i.raw
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
            self.message = "正在停止制作；临时文件清理完成后可关闭窗口。".into();
        }
        self.header(ctx);
        self.sidebar(ctx);
        self.preview_panel(ctx);
        self.footer(ctx);
        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(BASE)
                    .inner_margin(Margin::symmetric(28, 20)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("main-scroll")
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("C O L L E C T .   B I N D .   R E A D .")
                                .size(9.0)
                                .color(IRIS),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("让每一页，都有归处。")
                                .size(28.0)
                                .color(TEXT)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("将喜欢的图片，装订成随时可以翻阅的书。")
                                .size(12.0)
                                .color(SUBTLE),
                        );
                        ui.add_space(9.0);
                        ui.add_enabled_ui(!self.busy, |ui| self.drop_zone(ui, ctx));
                        ui.add_space(6.0);
                        self.queue(ui);
                        ui.add_space(6.0);
                        self.settings(ui, ctx);
                        if !self.message.is_empty() {
                            ui.add_space(5.0);
                            ui.label(RichText::new(&self.message).size(11.0).color(
                                if self.message.starts_with("制作结束") {
                                    FOAM
                                } else {
                                    GOLD
                                },
                            ));
                        }
                        if let Some(result) = self.tasks.iter().rev().find_map(|t| {
                            if let Status::Done(r) = &t.status {
                                Some(r)
                            } else {
                                None
                            }
                        }) && ui.button("打开输出文件夹 ↗").clicked()
                            && let Some(parent) = result.path.parent()
                            && let Err(e) = open::that_detached(parent)
                        {
                            self.message = format!("无法打开文件夹：{e}");
                        }
                    });
            });
        if self.help {
            egui::Window::new("使用指南")
                .open(&mut self.help)
                .resizable(false)
                .default_width(475.0)
                .show(ctx, |ui| {
                    ui.label(RichText::new("三步，装订一本新书。").size(21.0).color(ROSE));
                    ui.add_space(10.0);
                    for line in [
                        "01  拖入文件，或点击「选择文件 / 选择文件夹」。",
                        "02  选择 EPUB、PDF 或 CBZ，设置书名与输出位置。",
                        "03  点击「开始制作」，完成后打开输出文件夹。",
                        "",
                        "• 多个压缩包各生成一本书；一起选择的单张图片合成一本。",
                        "• 文件夹中的图片（含子文件夹）合成一本；其中的压缩包各生成一本。",
                        "• EPUB 使用固定版式、一图一页，第一张图片作为封面。",
                        "• WebP 等图片在 EPUB 中转为 JPEG；CBZ 保留图片原始字节。",
                        "• GIF / 多页 TIFF 在 EPUB / PDF 中只使用第一帧 / 第一页。",
                        "• 文件名自然排序。加密压缩包、嵌套压缩包暂不支持。",
                        "• 7Z / RAR / TAR 在制作前显示示意封面，不重复解压预览。",
                        "• 转换失败可查看队列提示并重试；停止制作会清理临时文件。",
                        "• 输出到 Telysta Books；同名书籍自动编号，原文件不会改动。",
                    ] {
                        ui.label(RichText::new(line).size(12.0).color(SUBTLE));
                    }
                });
        }
        // Opt-in visual regression capture using eframe's native screenshot event.
        if let Ok(path) = std::env::var("TELYSTA_SCREENSHOT") {
            let delay_ms = std::env::var("TELYSTA_SCREENSHOT_AFTER_MS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1000);
            if !self.screenshot_requested
                && self.screenshot_started.elapsed().as_millis() >= u128::from(delay_ms)
            {
                self.screenshot_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            let capture = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(image) = capture {
                let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                let _ = image::save_buffer(
                    &path,
                    &bytes,
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

fn pill(ui: &mut egui::Ui, text: &str, color: Color32) {
    Frame::new()
        .fill(OVERLAY)
        .corner_radius(5)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(10.0).color(color));
        });
}

fn draw_book(p: &egui::Painter, r: Rect, color: Color32) {
    let c = r.center().x;
    let s = Stroke::new(1.5_f32, color);
    p.line_segment([Pos2::new(c, r.top() + 2.0), Pos2::new(c, r.bottom())], s);
    for (edge, delta) in [(r.left(), -2.0), (r.right(), 2.0)] {
        p.line_segment([Pos2::new(c, r.top() + 2.0), Pos2::new(edge, r.top())], s);
        p.line_segment(
            [Pos2::new(edge, r.top()), Pos2::new(edge, r.bottom() - 2.0)],
            s,
        );
        p.line_segment(
            [Pos2::new(edge, r.bottom() - 2.0), Pos2::new(c, r.bottom())],
            s,
        );
        p.line_segment(
            [
                Pos2::new(c + delta, r.top() + 6.0),
                Pos2::new(edge - delta, r.top() + 5.0),
            ],
            Stroke::new(1.0_f32, color),
        );
    }
}

fn draw_cover(p: &egui::Painter, r: Rect) {
    p.rect_filled(r, 4, Color32::from_rgb(0x30, 0x2a, 0x42));
    p.rect_filled(
        Rect::from_min_max(r.min, Pos2::new(r.min.x + 9.0, r.max.y)),
        2,
        Color32::from_rgb(0x26, 0x21, 0x36),
    );
    let inner = r.shrink2(Vec2::new(25.0, 22.0));
    p.rect_stroke(
        inner,
        0,
        Stroke::new(0.6_f32, Color32::from_rgb(0x67, 0x58, 0x72)),
        StrokeKind::Inside,
    );
    p.text(
        Pos2::new(r.center().x, r.top() + 38.0),
        Align2::CENTER_CENTER,
        "T H E   Q U I E T   C O L L E C T I O N",
        FontId::proportional(6.5),
        ROSE,
    );
    p.text(
        Pos2::new(r.center().x, r.top() + 82.0),
        Align2::CENTER_CENTER,
        "每一页",
        FontId::proportional(28.0),
        TEXT,
    );
    p.text(
        Pos2::new(r.center().x, r.top() + 117.0),
        Align2::CENTER_CENTER,
        "皆是珍藏",
        FontId::proportional(25.0),
        ROSE,
    );
    let center = Pos2::new(r.center().x, r.top() + 195.0);
    p.circle_stroke(
        center,
        39.0,
        Stroke::new(0.7_f32, Color32::from_rgb(0x77, 0x66, 0x83)),
    );
    p.circle_filled(center + Vec2::new(22.0, -25.0), 5.0, GOLD);
    let stem = center + Vec2::new(-6.0, 26.0);
    p.line_segment(
        [stem, stem + Vec2::new(13.0, -51.0)],
        Stroke::new(1.0_f32, FOAM),
    );
    for (y, left) in [(0.0, true), (-10.0, false), (-20.0, true), (-30.0, false)] {
        let base = stem + Vec2::new(-y / 4.0, y);
        let tip = base + Vec2::new(if left { -18.0 } else { 18.0 }, -13.0);
        let mid = (base + tip.to_vec2()) / 2.0;
        p.add(egui::Shape::convex_polygon(
            vec![
                base,
                mid + Vec2::new(0.0, -6.0),
                tip,
                mid + Vec2::new(0.0, 3.0),
            ],
            if left {
                FOAM.gamma_multiply(0.7)
            } else {
                IRIS.gamma_multiply(0.75)
            },
            Stroke::NONE,
        ));
    }
    p.text(
        Pos2::new(r.center().x, r.bottom() - 34.0),
        Align2::CENTER_CENTER,
        "B O U N D   W I T H   T E L Y S T A",
        FontId::proportional(6.5),
        SUBTLE,
    );
}
