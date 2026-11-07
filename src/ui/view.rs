use super::{components::*, theme::*, *};
use eframe::egui::{
    Align, Align2, FontId, Frame, Layout, Margin, Pos2, Rect, RichText, Sense, Stroke, StrokeKind,
    Vec2,
};

pub(super) fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "noto-sc".into(),
        egui::FontData::from_static(include_bytes!("../../assets/fonts/NotoSansSC.ttf")).into(),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "noto-sc".into());
    // Use the platform's Latin UI face when available, with embedded CJK fallback.
    #[cfg(target_os = "windows")]
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\segoeui.ttf") {
        fonts
            .font_data
            .insert("system-ui".into(), egui::FontData::from_owned(bytes).into());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "system-ui".into());
    }
    ctx.set_fonts(fonts);
}

fn heading(ui: &mut egui::Ui, title: &str, p: Palette) {
    ui.label(RichText::new(title).size(TYPE_SECTION).color(p.text));
}

fn panel(p: Palette) -> Frame {
    Frame::new()
        .fill(p.secondary_surface)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(PANEL_RADIUS)
        .inner_margin(PANEL_PADDING)
}

impl TelystaApp {
    pub(super) fn render(&mut self, ctx: &egui::Context) {
        let p = self.appearance.palette();
        self.toolbar(ctx, p);
        self.action_bar(ctx, p);
        let narrow = ctx.content_rect().width() < 1000.0;
        egui::CentralPanel::default()
            .frame(Frame::new().fill(p.canvas).inner_margin(if narrow {
                COMPACT_PADDING
            } else {
                PAGE_PADDING
            }))
            .show(ctx, |ui| {
                paint_canvas(ui.painter(), ui.max_rect(), p);
                let width = ui.available_width().min(CONTENT_WIDTH);
                let inset = ((ui.available_width() - width) / 2.0).max(0.0);
                ui.horizontal_top(|ui| {
                    ui.add_space(inset);
                    ui.allocate_ui_with_layout(
                        Vec2::new(width, ui.available_height()),
                        Layout::top_down(Align::LEFT),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("workspace-scroll")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.set_width(width);
                                    ui.spacing_mut().item_spacing.y = SPACE_XS;
                                    ui.label(
                                        RichText::new("漫画成书")
                                            .font(FontId::proportional(TYPE_TITLE))
                                            .strong()
                                            .color(p.text),
                                    );
                                    ui.label(
                                        RichText::new("导入本地漫画图片，按编号整理成电子书。")
                                            .size(TYPE_CONTROL)
                                            .color(p.secondary),
                                    );
                                    ui.add_space(SPACE_SECTION);
                                    if width >= COMPACT_BREAKPOINT {
                                        let left_width = width - INSPECTOR_WIDTH - COLUMN_GAP;
                                        let queue_height = (ctx.content_rect().height() - 380.0)
                                            .clamp(180.0, 480.0);
                                        ui.horizontal_top(|ui| {
                                            ui.allocate_ui_with_layout(
                                                Vec2::new(left_width, 1.0),
                                                Layout::top_down(Align::LEFT),
                                                |ui| self.materials(ui, ctx, p, queue_height),
                                            );
                                            ui.add_space(COLUMN_GAP - 12.0);
                                            ui.allocate_ui_with_layout(
                                                Vec2::new(INSPECTOR_WIDTH, 1.0),
                                                Layout::top_down(Align::LEFT),
                                                |ui| self.output_settings(ui, ctx, p),
                                            );
                                        });
                                    } else {
                                        self.materials(ui, ctx, p, 220.0);
                                        ui.add_space(SPACE_SECTION);
                                        self.output_settings(ui, ctx, p);
                                    }
                                    if !self.message.is_empty()
                                        && !self.message.starts_with("制作结束")
                                    {
                                        ui.add_space(SPACE_MD);
                                        Frame::new()
                                            .fill(p.surface)
                                            .corner_radius(CONTROL_RADIUS)
                                            .inner_margin(16)
                                            .show(ui, |ui| {
                                                ui.set_width(ui.available_width());
                                                ui.label(
                                                    RichText::new(&self.message)
                                                        .size(TYPE_HELPER)
                                                        .color(p.warning),
                                                );
                                            });
                                    }
                                });
                        },
                    );
                });
            });
        self.dialogs(ctx, p);
    }

    fn toolbar(&mut self, ctx: &egui::Context, p: Palette) {
        egui::TopBottomPanel::top("toolbar")
            .exact_height(60.0)
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .inner_margin(Margin::symmetric(PAGE_PADDING, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                    ui.painter().rect_filled(rect, 9, p.soft);
                    icon(ui.painter(), rect.shrink(8.0), Icon::Book, p.primary);
                    ui.label(RichText::new("Telysta").size(21.0).color(p.text));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("/  图片成书")
                            .size(TYPE_HELPER)
                            .color(p.secondary),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (kind, label) = if self.appearance == Appearance::Campus {
                            (Icon::Moon, "切换到深色")
                        } else {
                            (Icon::Sun, "切换到浅色")
                        };
                        let response = icon_button(ui, p, kind, label);
                        let mut chosen = self.appearance;
                        if response.clicked() {
                            chosen = if chosen == Appearance::Campus {
                                Appearance::Night
                            } else {
                                Appearance::Campus
                            };
                        }
                        response.context_menu(|ui| {
                            for (appearance, name) in [
                                (Appearance::Campus, "校园浅色"),
                                (Appearance::Night, "校园深色"),
                                (Appearance::RosePine, "Rosé Pine"),
                            ] {
                                if ui.selectable_value(&mut chosen, appearance, name).clicked() {
                                    ui.close();
                                }
                            }
                        });
                        if chosen != self.appearance {
                            self.appearance = chosen;
                            chosen.apply(ctx);
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("使用指南")
                                        .size(TYPE_HELPER)
                                        .color(p.secondary),
                                )
                                .frame(false)
                                .min_size(Vec2::new(72.0, CONTROL_HEIGHT)),
                            )
                            .clicked()
                        {
                            self.help = true;
                        }
                    });
                });
            });
    }

    fn materials(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, p: Palette, queue_height: f32) {
        ui.spacing_mut().item_spacing.y = SPACE_SM;
        self.drop_zone(ui, ctx, p);
        if self.tasks.is_empty() {
            ui.add_space(SPACE_SM);
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                icon(ui.painter(), r, Icon::Check, p.secondary);
                ui.label(
                    RichText::new("全部在本机处理，原文件会完整保留。")
                        .size(TYPE_HELPER)
                        .color(p.secondary),
                );
            });
        } else {
            ui.add_space(SPACE_LG);
            ui.horizontal(|ui| {
                heading(ui, "制作队列", p);
                ui.label(
                    RichText::new(format!("{} 本", self.tasks.len()))
                        .size(TYPE_HELPER)
                        .color(p.secondary),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            !self.busy,
                            egui::Button::new(
                                RichText::new("清空").size(TYPE_HELPER).color(p.secondary),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.tasks.clear();
                        self.selected = None;
                        self.message.clear();
                    }
                    if self
                        .tasks
                        .iter()
                        .any(|task| Some(task.id) == self.selected && task.thumbnail.is_some())
                        && tertiary_button(ui, p, "预览封面").clicked()
                    {
                        self.preview_open = true;
                    }
                });
            });
            self.queue(ui, p, queue_height);
        }
    }

    fn drop_zone(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, p: Palette) {
        let hovering = ctx.input(|input| !input.raw.hovered_files.is_empty()) && !self.busy;
        let width = ui.available_width();
        let enabled = !self.busy && !self.importing;
        let frame = Frame::new()
            .fill(if hovering { p.soft } else { p.surface })
            .stroke(Stroke::new(
                if hovering { 1.5_f32 } else { 1.0 },
                if hovering { p.focus } else { p.border_strong },
            ))
            .corner_radius(16)
            .inner_margin(if self.tasks.is_empty() { 24 } else { 16 });
        if self.tasks.is_empty() {
            frame.show(ui, |ui| {
                ui.set_width((width - 50.0).max(240.0));
                let artwork = ui.available_width() >= 540.0;
                ui.horizontal_centered(|ui| {
                    let text_width =
                        (ui.available_width() - if artwork { 200.0 } else { 0.0 }).max(252.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(text_width, 180.0),
                        Layout::top_down(Align::LEFT),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = SPACE_XS;
                            ui.add_space(SPACE_LG);
                            ui.label(
                                RichText::new(if self.importing {
                                    "正在整理漫画素材…"
                                } else if hovering {
                                    "松开即可添加"
                                } else {
                                    "导入漫画素材"
                                })
                                .size(21.0)
                                .strong()
                                .color(p.text),
                            );
                            ui.label(
                                RichText::new("选择压缩包或图片目录，也可直接拖入。 ")
                                    .size(TYPE_CONTROL)
                                    .color(p.secondary),
                            );
                            ui.add_space(SPACE_MD);
                            ui.horizontal(|ui| {
                                if primary_button(
                                    ui,
                                    p,
                                    "选择文件",
                                    Vec2::new(120.0, CONTROL_HEIGHT),
                                    enabled,
                                )
                                .on_hover_text("Ctrl / ⌘ + O")
                                .clicked()
                                {
                                    self.pick(false, ctx);
                                }
                                if ui
                                    .add_enabled(
                                        enabled,
                                        egui::Button::new("选择文件夹")
                                            .min_size(Vec2::new(120.0, CONTROL_HEIGHT)),
                                    )
                                    .clicked()
                                {
                                    self.pick(true, ctx);
                                }
                            });
                        },
                    );
                    if artwork {
                        let (rect, _) =
                            ui.allocate_exact_size(Vec2::new(180.0, 180.0), Sense::hover());
                        source_art(ui.painter(), rect, p);
                    }
                });
            });
        } else {
            frame.show(ui, |ui| {
                ui.set_width((width - 34.0).max(240.0));
                ui.horizontal_centered(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 238.0).max(150.0));
                        ui.label(RichText::new("漫画素材").size(TYPE_BODY).color(p.text));
                        ui.label(
                            RichText::new(if self.importing {
                                "正在读取…"
                            } else if hovering {
                                "松开以添加到队列"
                            } else {
                                "继续添加，或拖入更多漫画"
                            })
                            .size(TYPE_HELPER)
                            .color(p.secondary),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Button::new("添加文件夹")
                                    .min_size(Vec2::new(104.0, CONTROL_HEIGHT)),
                            )
                            .clicked()
                        {
                            self.pick(true, ctx);
                        }
                        if ui
                            .add_enabled(
                                enabled,
                                egui::Button::new("添加文件")
                                    .min_size(Vec2::new(104.0, CONTROL_HEIGHT)),
                            )
                            .clicked()
                        {
                            self.pick(false, ctx);
                        }
                    });
                });
            });
        }
    }

    fn queue(&mut self, ui: &mut egui::Ui, p: Palette, height: f32) {
        let mut remove = None;
        let mut preview = false;
        let mut error = None;
        Frame::new()
            .fill(p.surface)
            .stroke(Stroke::new(1.0_f32, p.border))
            .corner_radius(PANEL_RADIUS)
            .inner_margin(8)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("book-list")
                    .max_height(height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for task in &self.tasks {
                            let selected = self.selected == Some(task.id);
                            let (rect, response) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), ROW_HEIGHT),
                                Sense::click(),
                            );
                            let response =
                                response.on_hover_text(task.source.path().display().to_string());
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::SelectableLabel,
                                    ui.is_enabled(),
                                    selected,
                                    &task.title,
                                )
                            });
                            if selected || response.hovered() {
                                ui.painter().rect_filled(
                                    rect,
                                    CONTROL_RADIUS,
                                    if selected { p.soft } else { p.hover },
                                );
                            }
                            if selected {
                                ui.painter().rect_filled(
                                    Rect::from_min_size(
                                        rect.min + Vec2::new(0.0, 20.0),
                                        Vec2::new(3.0, 40.0),
                                    ),
                                    2,
                                    p.primary,
                                );
                            }
                            if response.has_focus() {
                                ui.painter().rect_stroke(
                                    rect.shrink(1.0),
                                    CONTROL_RADIUS,
                                    Stroke::new(1.5_f32, p.focus),
                                    StrokeKind::Inside,
                                );
                            }
                            let cover = Rect::from_min_size(
                                rect.min + Vec2::new(16.0, 14.0),
                                Vec2::new(38.0, 52.0),
                            );
                            if let Some(texture) = &task.thumbnail {
                                ui.painter().rect_filled(cover, 4, p.canvas);
                                let size = texture.size_vec2();
                                let scale = (cover.width() / size.x).min(cover.height() / size.y);
                                ui.painter().image(
                                    texture.id(),
                                    Rect::from_center_size(cover.center(), size * scale),
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    egui::Color32::WHITE,
                                );
                            } else {
                                ui.painter().rect_filled(cover, 4, p.canvas);
                                icon(
                                    ui.painter(),
                                    cover.shrink2(Vec2::new(9.0, 16.0)),
                                    Icon::Book,
                                    p.secondary,
                                );
                            }
                            let (detail, color) = match &task.status {
                                Status::Pending => {
                                    (format!("{} · 等待制作", task.source.kind()), p.secondary)
                                }
                                Status::Running(progress) => (progress.message.clone(), p.primary),
                                Status::Done(result) => (
                                    format!(
                                        "已完成 · {} 页 · {:.1} MiB",
                                        result.pages,
                                        result.bytes as f64 / 1_048_576.0
                                    ),
                                    p.success,
                                ),
                                Status::Failed(_) => ("制作失败 · 点击查看详情".into(), p.danger),
                                Status::Cancelled => ("已停止 · 可以重新制作".into(), p.warning),
                            };
                            let text_rect = Rect::from_min_max(
                                rect.min + Vec2::new(68.0, 16.0),
                                rect.max - Vec2::new(60.0, 12.0),
                            );
                            let painter = ui.painter().with_clip_rect(text_rect);
                            painter.text(
                                text_rect.left_top(),
                                Align2::LEFT_TOP,
                                &task.title,
                                FontId::proportional(TYPE_BODY),
                                p.text,
                            );
                            painter.text(
                                text_rect.left_top() + Vec2::new(0.0, 28.0),
                                Align2::LEFT_TOP,
                                detail,
                                FontId::proportional(TYPE_META),
                                color,
                            );
                            let close_rect = Rect::from_center_size(
                                Pos2::new(rect.right() - 28.0, rect.center().y),
                                Vec2::splat(CONTROL_HEIGHT),
                            );
                            let close = ui.put(
                                close_rect,
                                egui::Button::new("").frame(false).sense(if self.busy {
                                    Sense::hover()
                                } else {
                                    Sense::click()
                                }),
                            );
                            close.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    !self.busy,
                                    "移除书籍",
                                )
                            });
                            icon(
                                ui.painter(),
                                close_rect.shrink(12.0),
                                Icon::Close,
                                if close.hovered() && !self.busy {
                                    p.danger
                                } else {
                                    p.muted
                                },
                            );
                            if close.has_focus() {
                                ui.painter().rect_stroke(
                                    close_rect.shrink(2.0),
                                    CONTROL_RADIUS,
                                    Stroke::new(1.5_f32, p.focus),
                                    StrokeKind::Inside,
                                );
                            }
                            if close.on_hover_text("移除这本书").clicked() && !self.busy {
                                remove = Some(task.id);
                            }
                            if response.clicked() {
                                self.selected = Some(task.id);
                                if let Status::Failed(message) = &task.status {
                                    error = Some(message.clone());
                                }
                            }
                            if response.double_clicked() && task.thumbnail.is_some() {
                                self.selected = Some(task.id);
                                preview = true;
                            }
                            response.context_menu(|ui| {
                                if ui
                                    .add_enabled(
                                        task.thumbnail.is_some(),
                                        egui::Button::new("查看封面"),
                                    )
                                    .clicked()
                                {
                                    self.selected = Some(task.id);
                                    preview = true;
                                    ui.close();
                                }
                                if let Status::Done(result) = &task.status
                                    && ui.button("打开书籍").clicked()
                                {
                                    let _ = open::that_detached(&result.path);
                                    ui.close();
                                }
                                if let Status::Failed(message) = &task.status
                                    && ui.button("查看错误详情").clicked()
                                {
                                    error = Some(message.clone());
                                    ui.close();
                                }
                            });
                        }
                    });
            });
        if let Some(id) = remove {
            self.tasks.retain(|task| task.id != id);
            if self.selected == Some(id) {
                self.selected = self.tasks.first().map(|task| task.id);
            }
        }
        if preview {
            self.preview_open = true;
        }
        if error.is_some() {
            self.error_details = error;
        }
    }

    fn output_settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, p: Palette) {
        let width = ui.available_width();
        panel(p).show(ui, |ui| {
            ui.set_width((width - 42.0).max(200.0));
            ui.spacing_mut().item_spacing.y = SPACE_XS;
            heading(ui, "输出设置", p);
            ui.add_space(SPACE_XS);
            ui.add_enabled_ui(!self.busy, |ui| {
                ui.label(
                    RichText::new("书籍格式")
                        .size(TYPE_HELPER)
                        .color(p.secondary),
                );
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = SPACE_XS;
                    let button_width = (ui.available_width() - 16.0) / 3.0;
                    for format in BookFormat::ALL {
                        let selected = self.format == format;
                        let response = ui.add_sized(
                            Vec2::new(button_width, CONTROL_HEIGHT),
                            egui::Button::new(
                                RichText::new(format.label())
                                    .size(TYPE_CONTROL)
                                    .color(if selected { p.primary } else { p.secondary }),
                            )
                            .fill(if selected { p.soft } else { p.surface })
                            .stroke(Stroke::new(
                                if selected { 1.5_f32 } else { 1.0 },
                                if selected { p.primary } else { p.border_strong },
                            ))
                            .corner_radius(CONTROL_RADIUS),
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::RadioButton,
                                ui.is_enabled(),
                                selected,
                                format.label(),
                            )
                        });
                        if response.clicked() {
                            self.set_format(format);
                        }
                    }
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(match self.format {
                        BookFormat::Epub => "一图一页，适合电子书阅读器。",
                        BookFormat::Pdf => "按原图比例排版，便于分享和打印。",
                        BookFormat::Cbz => "保留原始图片，适合漫画阅读器。",
                    })
                    .size(TYPE_META)
                    .color(p.secondary),
                );
                if let Some(task) = self
                    .tasks
                    .iter_mut()
                    .find(|task| Some(task.id) == self.selected)
                {
                    ui.add_space(SPACE_SM);
                    ui.label(RichText::new("书名").size(TYPE_HELPER).color(p.secondary));
                    ui.add(
                        egui::TextEdit::singleline(&mut task.title)
                            .desired_width(f32::INFINITY)
                            .margin(Vec2::new(10.0, 9.0)),
                    );
                    if task.title.trim().is_empty() {
                        ui.label(
                            RichText::new("请填写书名后开始制作。")
                                .size(TYPE_META)
                                .color(p.danger),
                        );
                    }
                }
                ui.add_space(SPACE_SM);
                ui.label(
                    RichText::new("保存位置")
                        .size(TYPE_HELPER)
                        .color(p.secondary),
                );
                let location = if let Some(parent) = &self.output_parent {
                    parent
                        .file_name()
                        .unwrap_or(parent.as_os_str())
                        .to_string_lossy()
                        .to_string()
                } else {
                    "源文件所在目录".into()
                };
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                    icon(ui.painter(), rect, Icon::Folder, p.secondary);
                    ui.add(
                        egui::Label::new(RichText::new(&location).size(TYPE_HELPER).color(p.text))
                            .truncate(),
                    )
                    .on_hover_text(self.output_path());
                });
                ui.label(
                    RichText::new("自动新建 Telysta Books 文件夹")
                        .size(TYPE_META)
                        .color(p.secondary),
                );
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.choosing_output,
                            egui::Button::new(
                                RichText::new("更改位置").size(TYPE_HELPER).color(p.primary),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        self.choose_output(ctx);
                    }
                    if self.output_parent.is_some() && tertiary_button(ui, p, "恢复默认").clicked()
                    {
                        self.output_parent = None;
                    }
                });
                ui.add_space(SPACE_XS);
                egui::CollapsingHeader::new(
                    RichText::new("更多选项")
                        .size(TYPE_HELPER)
                        .color(p.secondary),
                )
                .id_salt("advanced-options")
                .default_open(false)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 10.0;
                    ui.label(
                        RichText::new("同时制作")
                            .size(TYPE_HELPER)
                            .color(p.secondary),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.parallelism, 1..=telysta::batch::MAX_JOBS)
                            .suffix(" 本"),
                    );
                    ui.label(
                        RichText::new("多本漫画可以并发制作，单本页序保持不变。")
                            .size(TYPE_META)
                            .color(p.secondary),
                    );
                    ui.add_space(SPACE_SM);
                    ui.label(RichText::new("作者").size(TYPE_HELPER).color(p.secondary));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.author)
                            .hint_text("留空为佚名")
                            .desired_width(f32::INFINITY)
                            .margin(Vec2::new(10.0, 9.0)),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("阅读方向")
                            .size(TYPE_HELPER)
                            .color(p.secondary),
                    );
                    egui::ComboBox::from_id_salt("reading-direction")
                        .width(ui.available_width() - 10.0)
                        .selected_text(if self.direction == Direction::Ltr {
                            "从左至右"
                        } else {
                            "从右至左"
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.direction, Direction::Ltr, "从左至右");
                            ui.selectable_value(&mut self.direction, Direction::Rtl, "从右至左");
                        });
                    if self.format != BookFormat::Cbz {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("JPEG 转码质量")
                                .size(TYPE_HELPER)
                                .color(p.secondary),
                        );
                        ui.add(egui::Slider::new(&mut self.quality, 60..=100).suffix("%"));
                        ui.label(
                            RichText::new(if self.format == BookFormat::Epub {
                                "原始 JPEG / PNG 通常直接保留。"
                            } else {
                                "图片统一以 JPEG 嵌入 PDF。"
                            })
                            .size(TYPE_META)
                            .color(p.secondary),
                        );
                    }
                });
            });
        });
    }

    fn choose_output(&mut self, ctx: &egui::Context) {
        self.choosing_output = true;
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        thread::spawn(move || {
            let path = rfd::FileDialog::new()
                .set_title("选择保存位置（会自动新建 Telysta Books）")
                .pick_folder();
            let _ = tx.send(Event::PickedOutput(path));
            ctx.request_repaint();
        });
    }

    fn action_bar(&mut self, ctx: &egui::Context, p: Palette) {
        let remaining = self
            .tasks
            .iter()
            .filter(|task| !matches!(task.status, Status::Done(_)))
            .count();
        let done = self
            .tasks
            .iter()
            .filter(|task| matches!(task.status, Status::Done(_)))
            .count();
        egui::TopBottomPanel::bottom("actions")
            .exact_height(80.0)
            .frame(
                Frame::new()
                    .fill(p.surface)
                    .inner_margin(Margin::symmetric(PAGE_PADDING, 16)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let summary_width = (ui.available_width()
                        - if done > 0 && !self.busy { 332.0 } else { 200.0 })
                    .max(180.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(summary_width, 48.0),
                        Layout::top_down(Align::LEFT),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 6.0;
                            if self.busy {
                                let mut fraction = 0.0;
                                let mut message = "等待制作…".to_owned();
                                for task in self
                                    .tasks
                                    .iter()
                                    .filter(|task| self.batch_ids.contains(&task.id))
                                {
                                    match &task.status {
                                        Status::Done(_) | Status::Failed(_) | Status::Cancelled => {
                                            fraction += 1.0
                                        }
                                        Status::Running(progress) => {
                                            fraction += progress.fraction;
                                            message.clone_from(&progress.message);
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
                                    .size(TYPE_CONTROL)
                                    .color(p.text),
                                );
                                ui.add(
                                    egui::ProgressBar::new(
                                        fraction / self.batch_ids.len().max(1) as f32,
                                    )
                                    .desired_height(5.0)
                                    .fill(p.primary)
                                    .corner_radius(3),
                                );
                            } else {
                                let summary = if self.tasks.is_empty() {
                                    "添加素材后，即可开始制作。".into()
                                } else if remaining == 0 {
                                    format!("已完成 {done} 本书籍")
                                } else {
                                    format!("已准备 {remaining} 本书籍")
                                };
                                ui.label(RichText::new(summary).size(TYPE_CONTROL).color(p.text));
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(if self.message.starts_with("制作结束") {
                                            self.message.as_str()
                                        } else {
                                            "原文件保留 · 同名书籍自动编号"
                                        })
                                        .size(TYPE_META)
                                        .color(p.secondary),
                                    )
                                    .truncate(),
                                );
                            }
                        },
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if self.busy {
                            if ui
                                .add_enabled(
                                    !self.cancel.load(Ordering::Relaxed),
                                    egui::Button::new("停止制作").min_size(Vec2::new(160.0, 44.0)),
                                )
                                .clicked()
                            {
                                self.cancel.store(true, Ordering::Relaxed);
                            }
                        } else if remaining == 0 && done > 0 {
                            if primary_button(ui, p, "打开输出文件夹", Vec2::new(172.0, 44.0), true)
                                .clicked()
                            {
                                self.open_output();
                            }
                            if tertiary_button(ui, p, "重新制作").clicked() {
                                for task in &mut self.tasks {
                                    task.status = Status::Pending;
                                }
                                self.message.clear();
                            }
                        } else {
                            let enabled = remaining > 0
                                && !self.importing
                                && !self.choosing_output
                                && self
                                    .tasks
                                    .iter()
                                    .filter(|task| !matches!(task.status, Status::Done(_)))
                                    .all(|task| !task.title.trim().is_empty());
                            let label = if remaining > 1 {
                                format!("开始制作 · {remaining} 本")
                            } else {
                                "开始制作".into()
                            };
                            if primary_button(ui, p, &label, Vec2::new(172.0, 44.0), enabled)
                                .on_hover_text("Ctrl+Enter")
                                .clicked()
                            {
                                self.start(ctx);
                            }
                            if done > 0 && tertiary_button(ui, p, "打开输出目录").clicked() {
                                self.open_output();
                            }
                        }
                    });
                });
            });
    }

    fn dialogs(&mut self, ctx: &egui::Context, p: Palette) {
        if self.help {
            egui::Window::new("使用指南").open(&mut self.help).default_width(480.0).resizable(false).collapsible(false).show(ctx,|ui| {
                ui.spacing_mut().item_spacing.y=16.0;
                for text in ["1. 添加图片、文件夹或压缩包。","2. 选择格式和保存位置，可为选中的书籍修改书名。","3. 点击「开始制作」，完成后打开输出文件夹。","多个压缩包各生成一本书，一起选择的单张图片合成一本。文件夹中的图片会递归收集；其中的压缩包各自成书。","图片按自然文件名排序。第一张图片作为 EPUB 封面；双击队列中的缩略图可以查看封面。","作者、阅读方向和转码质量位于「更多选项」。WebP 转 JPEG 可能增加文件体积；CBZ 保留原图。","GIF / 多页 TIFF 在 EPUB、PDF 中仅取第一帧 / 第一页。加密和嵌套压缩包暂不支持。","快捷键：Ctrl+O 选择文件，Ctrl+Shift+O 选择文件夹，Ctrl+Enter 开始制作，Esc 关闭说明和预览。"] {ui.label(RichText::new(text).size(TYPE_CONTROL).color(p.secondary));}
            });
        }
        if self.preview_open {
            let task = self
                .tasks
                .iter()
                .find(|task| Some(task.id) == self.selected);
            egui::Window::new("封面预览")
                .open(&mut self.preview_open)
                .default_width(400.0)
                .collapsible(false)
                .show(ctx, |ui| {
                    if let Some(task) = task {
                        ui.label(RichText::new(&task.title).size(16.0));
                        if let Some(texture) = &task.thumbnail {
                            ui.add(
                                egui::Image::new(texture)
                                    .max_size(Vec2::new(480.0, 560.0))
                                    .maintain_aspect_ratio(true),
                            );
                        } else {
                            ui.label("这份压缩包暂不提供封面预览。");
                        }
                    }
                });
        }
        if let Some(error) = &self.error_details {
            let mut open = true;
            egui::Window::new("制作失败").open(&mut open).default_width(500.0).collapsible(false).show(ctx,|ui| {
                ui.label(RichText::new(error).size(TYPE_CONTROL).color(p.danger));ui.add_space(SPACE_SM);ui.label(RichText::new("请检查文件是否完整、是否加密，以及保存位置是否有写入权限。修正后可重新制作。").size(TYPE_HELPER).color(p.secondary));
            });
            if !open {
                self.error_details = None;
            }
        }
    }

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        let (folder, files, start, escape) = ctx.input_mut(|input| {
            (
                input.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::O,
                )),
                input.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    egui::Key::O,
                )),
                input.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    egui::Key::Enter,
                )),
                input.key_pressed(egui::Key::Escape),
            )
        });
        if !self.busy && !self.importing {
            if folder {
                self.pick(true, ctx);
            } else if files {
                self.pick(false, ctx);
            }
        }
        if start
            && !self.busy
            && !self.importing
            && !self.choosing_output
            && self.tasks.iter().all(|task| !task.title.trim().is_empty())
        {
            self.start(ctx);
        }
        if escape {
            self.help = false;
            self.preview_open = false;
            self.error_details = None;
        }
    }

    pub(super) fn capture(&mut self, ctx: &egui::Context) {
        if let Ok(path) = std::env::var("TELYSTA_SCREENSHOT") {
            let delay = std::env::var("TELYSTA_SCREENSHOT_AFTER_MS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1000);
            if !self.screenshot_requested
                && self.screenshot_started.elapsed().as_millis() >= u128::from(delay)
            {
                self.screenshot_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            let screenshot = ctx.input(|input| {
                input.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(image) = screenshot {
                let bytes: Vec<u8> = image
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_array())
                    .collect();
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
}
