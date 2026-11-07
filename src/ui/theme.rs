use eframe::egui::{self, Color32, CornerRadius, FontId, Stroke, Vec2};

pub const CONTROL_HEIGHT: f32 = 40.0;
pub const PANEL_RADIUS: u8 = 12;
pub const CONTROL_RADIUS: u8 = 8;
pub const CONTENT_WIDTH: f32 = 1184.0;
pub const INSPECTOR_WIDTH: f32 = 304.0;
pub const COLUMN_GAP: f32 = 40.0;
pub const COMPACT_BREAKPOINT: f32 = 960.0;
pub const ROW_HEIGHT: f32 = 80.0;
pub const TYPE_TITLE: f32 = 32.0;
pub const TYPE_SECTION: f32 = 17.0;
pub const TYPE_BODY: f32 = 15.0;
pub const TYPE_CONTROL: f32 = 14.0;
pub const TYPE_HELPER: f32 = 13.0;
pub const TYPE_META: f32 = 12.0;
pub const SPACE_XS: f32 = 8.0;
pub const SPACE_SM: f32 = 12.0;
pub const SPACE_MD: f32 = 16.0;
pub const SPACE_LG: f32 = 20.0;
pub const SPACE_SECTION: f32 = 24.0;
pub const PAGE_PADDING: i8 = 32;
pub const COMPACT_PADDING: i8 = 20;
pub const PANEL_PADDING: i8 = 20;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    Campus,
    Night,
    RosePine,
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub canvas: Color32,
    pub canvas_alt: Color32,
    pub surface: Color32,
    pub secondary_surface: Color32,
    pub soft: Color32,
    pub hover: Color32,
    pub primary: Color32,
    pub primary_hover: Color32,
    pub primary_pressed: Color32,
    pub on_primary: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub muted: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub focus: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub cyan: Color32,
    pub lilac: Color32,
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

impl Appearance {
    pub fn palette(self) -> Palette {
        match self {
            Self::Campus => Palette {
                canvas: rgb(0xeff7fd),
                canvas_alt: rgb(0xe7f3fc),
                surface: Color32::WHITE,
                secondary_surface: rgb(0xf7fbff),
                soft: rgb(0xedf6fd),
                hover: rgb(0xf0f8fe),
                // Darkened from the reference blue: white button text exceeds 4.5:1.
                primary: rgb(0x1475ad),
                primary_hover: rgb(0x106493),
                primary_pressed: rgb(0x0d547b),
                on_primary: Color32::WHITE,
                text: rgb(0x203b53),
                secondary: rgb(0x5d7185),
                muted: rgb(0x748697),
                border: rgb(0xd6e7f4),
                border_strong: rgb(0xbbd8eb),
                focus: rgb(0x147dbb),
                success: rgb(0x1c7757),
                warning: rgb(0x9e6316),
                danger: rgb(0xb74452),
                cyan: rgb(0x56c9e8),
                lilac: rgb(0xb8bdf0),
            },
            Self::Night => Palette {
                canvas: rgb(0x111d2a),
                canvas_alt: rgb(0x162638),
                surface: rgb(0x1b2b3c),
                secondary_surface: rgb(0x21364a),
                soft: rgb(0x263f55),
                hover: rgb(0x2a465e),
                primary: rgb(0x69bdeb),
                primary_hover: rgb(0x83cdf3),
                primary_pressed: rgb(0x51a5d3),
                on_primary: rgb(0x111d2a),
                text: rgb(0xeaf4fb),
                secondary: rgb(0xb5c8d8),
                muted: rgb(0x91a9bc),
                border: rgb(0x344b60),
                border_strong: rgb(0x46657c),
                focus: rgb(0x83d0f5),
                success: rgb(0x75d6b7),
                warning: rgb(0xf3c77f),
                danger: rgb(0xf398ab),
                cyan: rgb(0x65cde5),
                lilac: rgb(0xb8bdf0),
            },
            Self::RosePine => Palette {
                canvas: rgb(0x191724),
                canvas_alt: rgb(0x232037),
                surface: rgb(0x1f1d2e),
                secondary_surface: rgb(0x242136),
                soft: rgb(0x26233a),
                hover: rgb(0x2b283f),
                primary: rgb(0xebbcba),
                primary_hover: rgb(0xf2ccca),
                primary_pressed: rgb(0xdca7a5),
                on_primary: rgb(0x191724),
                text: rgb(0xe0def4),
                secondary: rgb(0xaaa5c2),
                muted: rgb(0x908caa),
                border: rgb(0x343147),
                border_strong: rgb(0x524f67),
                focus: rgb(0xc4a7e7),
                success: rgb(0x9ccfd8),
                warning: rgb(0xf6c177),
                danger: rgb(0xeb6f92),
                cyan: rgb(0x9ccfd8),
                lilac: rgb(0xc4a7e7),
            },
        }
    }

    pub fn apply(self, ctx: &egui::Context) {
        let p = self.palette();
        let mut style = (*ctx.style()).clone();
        style.visuals = if self == Self::Campus {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        style.visuals.panel_fill = p.canvas;
        style.visuals.window_fill = p.surface;
        style.visuals.extreme_bg_color = p.surface;
        style.visuals.faint_bg_color = p.soft;
        style.visuals.override_text_color = Some(p.text);
        style.visuals.hyperlink_color = p.primary;
        style.visuals.selection.bg_fill = p.soft;
        style.visuals.selection.stroke = Stroke::new(1.5_f32, p.focus);
        style.visuals.window_stroke = Stroke::new(1.0_f32, p.border);
        style.visuals.window_corner_radius = CornerRadius::same(16);
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
        ] {
            widget.corner_radius = CornerRadius::same(CONTROL_RADIUS);
            widget.fg_stroke = Stroke::new(1.0_f32, p.text);
        }
        style.visuals.widgets.inactive.bg_fill = p.surface;
        style.visuals.widgets.inactive.weak_bg_fill = p.surface;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border_strong);
        style.visuals.widgets.hovered.bg_fill = p.hover;
        style.visuals.widgets.hovered.weak_bg_fill = p.hover;
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.focus);
        style.visuals.widgets.active.bg_fill = p.soft;
        style.visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, p.focus);
        style.spacing.item_spacing = Vec2::new(12.0, 12.0);
        style.spacing.button_padding = Vec2::new(16.0, 9.0);
        style.spacing.interact_size = Vec2::new(40.0, 40.0);
        style.animation_time = 0.15;
        style
            .text_styles
            .insert(egui::TextStyle::Heading, FontId::proportional(22.0));
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, FontId::proportional(13.0));
        ctx.set_style(style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn contrast(a: Color32, b: Color32) -> f32 {
        let luminance = |c: Color32| {
            let linear = |v: u8| {
                let v = v as f32 / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
        };
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
    #[test]
    fn essential_text_and_button_labels_have_readable_contrast() {
        for theme in [Appearance::Campus, Appearance::Night, Appearance::RosePine] {
            let p = theme.palette();
            for (foreground, background) in [
                (p.text, p.canvas),
                (p.text, p.surface),
                (p.secondary, p.canvas),
                (p.secondary, p.surface),
                (p.secondary, p.secondary_surface),
                (p.on_primary, p.primary),
                (p.on_primary, p.primary_hover),
                (p.on_primary, p.primary_pressed),
                (p.primary, p.soft),
                (p.success, p.surface),
                (p.success, p.soft),
                (p.warning, p.surface),
                (p.danger, p.surface),
                (p.danger, p.soft),
            ] {
                assert!(
                    contrast(foreground, background) >= 4.5,
                    "{theme:?}: {foreground:?} on {background:?}, contrast {}",
                    contrast(foreground, background)
                );
            }
        }
    }
}
