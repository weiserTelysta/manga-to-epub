use super::theme::*;
use eframe::egui::{
    self, Align2, Color32, FontId, Pos2, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};

#[derive(Clone, Copy)]
pub enum Icon {
    Book,
    Folder,
    Close,
    Sun,
    Moon,
    Image,
    Check,
}

pub fn paint_canvas(painter: &egui::Painter, rect: Rect, p: Palette) {
    let mut mesh = egui::epaint::Mesh::default();
    for (point, color) in [
        (rect.left_top(), p.secondary_surface),
        (rect.right_top(), p.canvas),
        (rect.right_bottom(), p.canvas_alt),
        (rect.left_bottom(), p.canvas),
    ] {
        mesh.colored_vertex(point, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
    let wash = Pos2::new(rect.right() - 80.0, rect.top() + 24.0);
    for step in (0..7).rev() {
        painter.circle_filled(
            wash,
            80.0 + step as f32 * 28.0,
            Color32::from_rgba_unmultiplied(p.cyan.r(), p.cyan.g(), p.cyan.b(), 3),
        );
    }
    if rect.width() > 900.0 {
        let origin = Pos2::new(rect.left() + 20.0, rect.bottom() - 130.0);
        for row in 0..8 {
            for col in 0..13 {
                painter.circle_filled(
                    origin + Vec2::new(col as f32 * 14.0, row as f32 * 14.0),
                    0.8,
                    Color32::from_rgba_unmultiplied(
                        p.primary.r(),
                        p.primary.g(),
                        p.primary.b(),
                        24,
                    ),
                );
            }
        }
        let center = Pos2::new(rect.left() - 80.0, rect.bottom() + 90.0);
        let arc = (0..=32)
            .map(|i| {
                let angle = -1.4 + i as f32 * 0.9 / 32.0;
                center + Vec2::angled(angle) * 250.0
            })
            .collect();
        painter.add(egui::Shape::line(
            arc,
            Stroke::new(
                1.0_f32,
                Color32::from_rgba_unmultiplied(p.primary.r(), p.primary.g(), p.primary.b(), 28),
            ),
        ));
    }
}

pub fn source_art(painter: &egui::Painter, rect: Rect, p: Palette) {
    let center = rect.center();
    painter.circle_filled(center, rect.height() * 0.43, p.soft);
    let page = Rect::from_center_size(center + Vec2::new(10.0, -2.0), Vec2::new(82.0, 112.0));
    painter.rect_filled(page.translate(Vec2::new(-19.0, 10.0)), 8, p.canvas_alt);
    painter.rect_stroke(
        page.translate(Vec2::new(-19.0, 10.0)),
        8,
        Stroke::new(1.0_f32, p.border_strong),
        StrokeKind::Inside,
    );
    painter.rect_filled(page, 8, p.surface);
    painter.rect_stroke(page, 8, Stroke::new(1.5_f32, p.primary), StrokeKind::Inside);
    icon(
        painter,
        Rect::from_center_size(page.center() - Vec2::new(0.0, 8.0), Vec2::splat(40.0)),
        Icon::Image,
        p.primary,
    );
    for (offset, width) in [(24.0, 50.0), (33.0, 34.0)] {
        painter.line_segment(
            [
                page.center() + Vec2::new(-25.0, offset),
                page.center() + Vec2::new(-25.0 + width, offset),
            ],
            Stroke::new(2.0_f32, p.border_strong),
        );
    }
    painter.line_segment(
        [
            center + Vec2::new(-65.0, -55.0),
            center + Vec2::new(-44.0, -55.0),
        ],
        Stroke::new(2.0_f32, p.cyan),
    );
    painter.circle_filled(center + Vec2::new(62.0, 53.0), 4.0, p.lilac);
}

pub fn icon(p: &egui::Painter, rect: Rect, kind: Icon, color: Color32) {
    let r = Rect::from_center_size(rect.center(), Vec2::splat(rect.width().min(rect.height())));
    let point = |x: f32, y: f32| {
        Pos2::new(
            r.left() + r.width() * x / 24.0,
            r.top() + r.height() * y / 24.0,
        )
    };
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment(
            [point(a.0, a.1), point(b.0, b.1)],
            Stroke::new(1.5_f32, color),
        );
    };
    match kind {
        Icon::Book => {
            for (a, b) in [
                ((3., 4.), (12., 6.)),
                ((12., 6.), (21., 4.)),
                ((3., 4.), (3., 19.)),
                ((21., 4.), (21., 19.)),
                ((3., 19.), (12., 21.)),
                ((12., 21.), (21., 19.)),
                ((12., 6.), (12., 21.)),
            ] {
                line(a, b);
            }
        }
        Icon::Folder => {
            for (a, b) in [
                ((3., 7.), (9., 7.)),
                ((9., 7.), (11., 9.)),
                ((11., 9.), (21., 9.)),
                ((21., 9.), (21., 20.)),
                ((21., 20.), (3., 20.)),
                ((3., 20.), (3., 7.)),
            ] {
                line(a, b);
            }
        }
        Icon::Close => {
            line((7., 7.), (17., 17.));
            line((7., 17.), (17., 7.));
        }
        Icon::Check => {
            line((5., 12.), (10., 17.));
            line((10., 17.), (20., 7.));
        }
        Icon::Image => {
            p.rect_stroke(
                Rect::from_min_max(point(3., 3.), point(21., 21.)),
                3,
                Stroke::new(1.5_f32, color),
                StrokeKind::Inside,
            );
            line((4., 17.), (9., 12.));
            line((9., 12.), (14., 17.));
            line((14., 17.), (17., 14.));
            line((17., 14.), (21., 18.));
            p.circle_stroke(point(15.5, 8.), 1.8, Stroke::new(1.5_f32, color));
        }
        Icon::Sun => {
            p.circle_stroke(r.center(), r.width() * 0.18, Stroke::new(1.5_f32, color));
            for (a, b) in [
                ((12., 2.), (12., 5.)),
                ((12., 19.), (12., 22.)),
                ((2., 12.), (5., 12.)),
                ((19., 12.), (22., 12.)),
                ((5., 5.), (7., 7.)),
                ((17., 17.), (19., 19.)),
                ((5., 19.), (7., 17.)),
                ((17., 7.), (19., 5.)),
            ] {
                line(a, b);
            }
        }
        Icon::Moon => {
            let points = [
                (17., 3.),
                (12., 4.),
                (9., 8.),
                (9., 13.),
                (12., 17.),
                (17., 18.),
                (21., 16.),
                (19., 20.),
                (14., 22.),
                (8., 20.),
                (4., 16.),
                (3., 10.),
                (6., 5.),
                (11., 2.),
                (17., 3.),
            ]
            .map(|v| point(v.0, v.1));
            p.add(egui::Shape::line(
                points.to_vec(),
                Stroke::new(1.5_f32, color),
            ));
        }
    }
}

pub fn icon_button(ui: &mut Ui, p: Palette, kind: Icon, label: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(CONTROL_HEIGHT), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, CONTROL_RADIUS, p.hover);
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            CONTROL_RADIUS,
            Stroke::new(1.5_f32, p.focus),
            StrokeKind::Inside,
        );
    }
    icon(
        ui.painter(),
        rect.shrink(10.0),
        kind,
        if response.hovered() {
            p.primary
        } else {
            p.secondary
        },
    );
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(label)
}

pub fn primary_button(ui: &mut Ui, p: Palette, label: &str, size: Vec2, enabled: bool) -> Response {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).size(TYPE_CONTROL).color(if enabled {
            p.on_primary
        } else {
            p.secondary
        }))
        .fill(if enabled { p.primary } else { p.soft })
        .corner_radius(CONTROL_RADIUS)
        .min_size(size),
    );
    if enabled
        && (response.hovered() || response.is_pointer_button_down_on() || response.has_focus())
    {
        let fill = if response.is_pointer_button_down_on() {
            p.primary_pressed
        } else {
            p.primary_hover
        };
        ui.painter()
            .rect_filled(response.rect, CONTROL_RADIUS, fill);
        ui.painter().text(
            response.rect.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(TYPE_CONTROL),
            p.on_primary,
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(3.0),
            CONTROL_RADIUS + 2,
            Stroke::new(1.5_f32, p.focus),
            StrokeKind::Inside,
        );
    }
    response
}

pub fn tertiary_button(ui: &mut Ui, p: Palette, label: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(label).size(TYPE_HELPER).color(p.primary))
            .frame(false)
            .min_size(Vec2::new(40.0, CONTROL_HEIGHT)),
    )
}
