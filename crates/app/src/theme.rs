//! Visual identity: palette, fonts, widget styling and the custom painted
//! pieces (checkbox, chips, strata bar and the layered landscape).
//!
//! The motif is geological: dev caches pile up like sediment, so sizes are
//! shown as stacked layers ("strata") in each category's colour.

use std::sync::Arc;

use cleaner_core::{CheckState, Category, Risk};
use eframe::egui::{
    self, epaint, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Pos2,
    Rect, Response, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2,
};

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    /// Window background.
    pub bg: Color32,
    /// Cards and rows.
    pub surface: Color32,
    /// Hovered rows, inputs.
    pub raised: Color32,
    pub line: Color32,
    pub text: Color32,
    pub muted: Color32,
    /// Sulfur yellow: marks what will be deleted and the main action.
    pub accent: Color32,
    pub accent_text: Color32,
    pub safe: Color32,
    pub review: Color32,
    pub danger: Color32,
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        if dark {
            Self {
                dark,
                bg: hex(0x161B24),
                surface: hex(0x1D2430),
                raised: hex(0x262F3D),
                line: hex(0x313B4A),
                text: hex(0xE7EAF0),
                muted: hex(0x8D97A7),
                accent: hex(0xF2C14E),
                accent_text: hex(0x231A02),
                safe: hex(0x5CC8A8),
                review: hex(0xF0A04B),
                danger: hex(0xE8636C),
            }
        } else {
            Self {
                dark,
                bg: hex(0xECEEF2),
                surface: hex(0xFFFFFF),
                raised: hex(0xF3F5F8),
                line: hex(0xD5DAE2),
                text: hex(0x1A212B),
                muted: hex(0x5E6979),
                accent: hex(0xF2C14E),
                accent_text: hex(0x231A02),
                safe: hex(0x1B9474),
                review: hex(0xB86512),
                danger: hex(0xC63F4A),
            }
        }
    }

    /// Accent tuned for text on the current background.
    pub fn accent_ink(&self) -> Color32 {
        if self.dark {
            self.accent
        } else {
            hex(0x9A6A00)
        }
    }

    pub fn risk(&self, r: Risk) -> Color32 {
        match r {
            Risk::Safe => self.safe,
            Risk::Review => self.review,
            Risk::Blocked => self.danger,
        }
    }
}

pub const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Opaque blend from `a` towards `b` (t = 0 gives `a`, 1 gives `b`). Used
/// instead of alpha so tints look the same on both themes.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

pub fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// One mineral-ish hue per category. Used for stripes, dots and strata.
pub fn category_color(c: Category) -> Color32 {
    match c {
        Category::Npm => hex(0xE0564E),
        Category::Yarn => hex(0x4FA3D9),
        Category::Pnpm => hex(0xF09A3A),
        Category::Bun => hex(0xE8C99B),
        Category::Pip => hex(0x4C7FC0),
        Category::Uv => hex(0xB08CF0),
        Category::Cargo => hex(0xD07A4C),
        Category::Gradle => hex(0x3FB39B),
        Category::Maven => hex(0xC94C82),
        Category::Go => hex(0x4CC6DB),
        Category::NuGet => hex(0x7383DE),
        Category::Composer => hex(0xA88563),
        Category::Docker => hex(0x2F8FE6),
        Category::Wsl => hex(0x9BC53D),
        Category::Ide => hex(0xE07BF0),
        Category::Temp => hex(0x8E98A8),
    }
}

// ---------------------------------------------------------------- fonts

pub const DISPLAY: &str = "display";
pub const STRONG: &str = "strong";

pub fn display(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(DISPLAY.into()))
}

pub fn strong(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(STRONG.into()))
}

pub fn body(size: f32) -> FontId {
    FontId::proportional(size)
}

fn load_first(paths: &[&str]) -> Option<Vec<u8>> {
    paths.iter().find_map(|p| std::fs::read(p).ok())
}

/// Uses system fonts when they exist (Bahnschrift for numbers and titles,
/// Segoe UI for text on Windows; DejaVu/Noto on Linux) and falls back to
/// egui's bundled fonts otherwise. Nothing is bundled into the binary.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let base_prop = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let win = |f: &str| format!("{windir}\\Fonts\\{f}");
    let (w_body, w_strong, w_display) = (win("segoeui.ttf"), win("seguisb.ttf"), win("bahnschrift.ttf"));

    let body_font = load_first(&[
        &w_body,
        "/System/Library/Fonts/SFNS.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]);
    let strong_font = load_first(&[
        &w_strong,
        "/usr/share/fonts/truetype/noto/NotoSans-SemiBold.ttf",
        "/usr/share/fonts/noto/NotoSans-SemiBold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    ]);
    let display_font = load_first(&[
        &w_display,
        "/usr/share/fonts/truetype/dejavu/DejaVuSansCondensed-Bold.ttf",
    ]);

    let mut prop = base_prop.clone();
    if let Some(bytes) = body_font {
        fonts
            .font_data
            .insert("cl-body".into(), Arc::new(FontData::from_owned(bytes)));
        prop.insert(0, "cl-body".into());
    }
    fonts.families.insert(FontFamily::Proportional, prop.clone());

    let mut strong_family = prop.clone();
    if let Some(bytes) = strong_font {
        fonts
            .font_data
            .insert("cl-strong".into(), Arc::new(FontData::from_owned(bytes)));
        strong_family.insert(0, "cl-strong".into());
    }
    fonts
        .families
        .insert(FontFamily::Name(STRONG.into()), strong_family.clone());

    let mut display_family = strong_family;
    if let Some(bytes) = display_font {
        fonts
            .font_data
            .insert("cl-display".into(), Arc::new(FontData::from_owned(bytes)));
        display_family.insert(0, "cl-display".into());
    }
    fonts
        .families
        .insert(FontFamily::Name(DISPLAY.into()), display_family);

    ctx.set_fonts(fonts);
}

// ---------------------------------------------------------------- style

pub fn apply(ctx: &egui::Context, p: &Palette) {
    let mut v = if p.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.bg;
    v.window_fill = p.surface;
    v.extreme_bg_color = p.raised;
    v.faint_bg_color = p.raised;
    v.code_bg_color = p.raised;
    v.window_stroke = Stroke::new(1.0_f32, p.line);
    v.window_corner_radius = CornerRadius::same(12);
    v.menu_corner_radius = CornerRadius::same(10);
    v.hyperlink_color = p.accent_ink();
    v.selection.bg_fill = mix(p.accent, p.bg, 0.6);
    v.selection.stroke = Stroke::new(1.0_f32, p.accent_ink());
    v.slider_trailing_fill = true;
    // Hand cursor on everything clickable (rows, tiles, menus, checkboxes…).
    v.interact_cursor = Some(egui::CursorIcon::PointingHand);
    v.window_shadow = epaint::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(if p.dark { 110 } else { 40 }),
    };
    v.popup_shadow = v.window_shadow;

    let radius = CornerRadius::same(8);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.surface;
    w.noninteractive.weak_bg_fill = p.surface;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.line);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.noninteractive.corner_radius = radius;

    w.inactive.bg_fill = p.raised;
    w.inactive.weak_bg_fill = p.raised;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, p.line);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.inactive.corner_radius = radius;

    w.hovered.bg_fill = p.raised;
    w.hovered.weak_bg_fill = p.raised;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, p.muted);
    w.hovered.fg_stroke = Stroke::new(1.5_f32, p.text);
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = p.line;
    w.active.weak_bg_fill = p.line;
    w.active.bg_stroke = Stroke::new(1.0_f32, p.accent_ink());
    w.active.fg_stroke = Stroke::new(1.5_f32, p.text);
    w.active.corner_radius = radius;
    w.active.expansion = 0.0;

    w.open = w.active;
    w.open.bg_stroke = Stroke::new(1.0_f32, p.line);

    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 8.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
        s.spacing.interact_size.y = 30.0;
        s.spacing.slider_width = 220.0;
        s.spacing.menu_margin = egui::Margin::same(8);
        s.text_styles = [
            (TextStyle::Heading, strong(22.0)),
            (TextStyle::Body, body(14.0)),
            (TextStyle::Button, body(14.0)),
            (TextStyle::Small, body(12.0)),
            (TextStyle::Monospace, FontId::monospace(12.0)),
        ]
        .into();
    });
}

// ---------------------------------------------------------------- widgets

/// Hover feedback shared by buttons and chips: the widget grows smoothly to
/// `max` while hovered and shrinks a little while pressed.
pub fn hover_scale(ui: &Ui, resp: &Response, max: f32) -> f32 {
    let enabled = resp.enabled();
    let t = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("hover-scale"), resp.hovered() && enabled, 0.12);
    let s = 1.0 + (max - 1.0) * t;
    if enabled && resp.is_pointer_button_down_on() {
        s * 0.96
    } else {
        s
    }
}

fn scaled(rect: Rect, s: f32) -> Rect {
    Rect::from_center_size(rect.center(), rect.size() * s)
}

/// Hand-drawn button so it can scale on hover. Disabled buttons keep their
/// shape but fade and do not react.
#[allow(clippy::too_many_arguments)]
fn button(
    ui: &mut Ui,
    p: &Palette,
    text: &str,
    font: FontId,
    fill: Color32,
    text_color: Color32,
    stroke: Stroke,
    enabled: bool,
) -> Response {
    let text = text.trim();
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font.clone(), text_color);
    let size = Vec2::new(galley.size().x + 40.0, 40.0);
    let resp = ui
        .add_enabled_ui(enabled, |ui| ui.allocate_exact_size(size, Sense::click()).1)
        .inner;
    let resp = if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp };
    let rect = resp.rect;
    if ui.is_rect_visible(rect) {
        let s = hover_scale(ui, &resp, 1.1);
        let r = scaled(rect, s);
        let (fill, text_color, stroke) = if enabled {
            (fill, text_color, stroke)
        } else {
            (
                mix(fill, p.bg, 0.55),
                mix(text_color, p.bg, 0.45),
                Stroke::new(stroke.width, mix(stroke.color, p.bg, 0.5)),
            )
        };
        let painter = ui.painter();
        painter.rect(r, CornerRadius::same((10.0 * s) as u8), fill, stroke, StrokeKind::Inside);
        let g = painter.layout_no_wrap(text.to_owned(), FontId::new(font.size * s, font.family), text_color);
        painter.galley(r.center() - g.size() / 2.0, g, text_color);
    }
    resp
}

pub fn primary_button(ui: &mut Ui, p: &Palette, text: &str, enabled: bool) -> Response {
    button(ui, p, text, strong(15.0), p.accent, p.accent_text, Stroke::NONE, enabled)
}

pub fn danger_button(ui: &mut Ui, p: &Palette, text: &str) -> Response {
    button(ui, p, text, strong(15.0), p.danger, Color32::WHITE, Stroke::NONE, true)
}

pub fn ghost_button(ui: &mut Ui, p: &Palette, text: &str) -> Response {
    let fill = if p.dark { Color32::TRANSPARENT } else { p.surface };
    button(ui, p, text, body(14.0), fill, p.text, Stroke::new(1.0_f32, p.line), true)
}

/// Rounded filter chip with an optional colour dot and trailing count.
pub fn chip(ui: &mut Ui, p: &Palette, label: &str, selected: bool, dot: Option<Color32>) -> Response {
    let font = body(13.0);
    let text_color = if selected { p.text } else { p.muted };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, text_color);
    let dot_w = if dot.is_some() { 14.0 } else { 0.0 };
    let size = Vec2::new(galley.size().x + 24.0 + dot_w, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let sc = hover_scale(ui, &resp, 1.05);
        let rect = scaled(rect, sc);
        let painter = ui.painter();
        let (fill, stroke) = if selected {
            (mix(p.accent, p.bg, if p.dark { 0.8 } else { 0.72 }), Stroke::new(1.0_f32, p.accent_ink()))
        } else if resp.hovered() {
            (p.raised, Stroke::new(1.0_f32, p.muted))
        } else {
            (Color32::TRANSPARENT, Stroke::new(1.0_f32, p.line))
        };
        painter.rect(rect, CornerRadius::same(14), fill, stroke, StrokeKind::Inside);
        let mut x = rect.left() + 12.0 * sc;
        if let Some(c) = dot {
            painter.circle_filled(Pos2::new(x + 4.0 * sc, rect.center().y), 4.0 * sc, c);
            x += dot_w * sc;
        }
        let galley = if sc != 1.0 {
            painter.layout_no_wrap(label.to_owned(), body(13.0 * sc), text_color)
        } else {
            galley
        };
        painter.galley(
            Pos2::new(x, rect.center().y - galley.size().y / 2.0),
            galley,
            text_color,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Small coloured label (risk level).
pub fn paint_pill(painter: &egui::Painter, right_center: Pos2, text: &str, color: Color32, p: &Palette) -> Rect {
    let galley = painter.layout_no_wrap(text.to_owned(), body(11.5), color);
    let size = Vec2::new(galley.size().x + 16.0, 20.0);
    let rect = Rect::from_min_size(
        Pos2::new(right_center.x - size.x, right_center.y - size.y / 2.0),
        size,
    );
    painter.rect_filled(rect, CornerRadius::same(10), mix(color, p.surface, if p.dark { 0.78 } else { 0.86 }));
    painter.galley(
        Pos2::new(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    rect
}

pub fn paint_checkbox(painter: &egui::Painter, rect: Rect, state: CheckState, enabled: bool, p: &Palette) {
    let r = CornerRadius::same(5);
    if !enabled {
        painter.rect(rect, r, p.raised, Stroke::new(1.0_f32, p.line), StrokeKind::Inside);
        // little padlock: shackle + body
        let c = rect.center();
        painter.circle_stroke(Pos2::new(c.x, c.y - 1.5), 3.0, Stroke::new(1.4_f32, p.muted));
        painter.rect_filled(
            Rect::from_center_size(Pos2::new(c.x, c.y + 2.5), Vec2::new(9.0, 6.0)),
            CornerRadius::same(1),
            p.muted,
        );
        return;
    }
    match state {
        CheckState::Unchecked => {
            painter.rect(rect, r, Color32::TRANSPARENT, Stroke::new(1.5_f32, p.muted), StrokeKind::Inside);
        }
        CheckState::Checked => {
            painter.rect_filled(rect, r, p.accent);
            let a = Pos2::new(rect.left() + rect.width() * 0.24, rect.center().y + 0.5);
            let b = Pos2::new(rect.left() + rect.width() * 0.43, rect.bottom() - rect.height() * 0.27);
            let c = Pos2::new(rect.right() - rect.width() * 0.22, rect.top() + rect.height() * 0.28);
            let s = Stroke::new(2.0_f32, p.accent_text);
            painter.line_segment([a, b], s);
            painter.line_segment([b, c], s);
        }
        CheckState::Partial => {
            painter.rect_filled(rect, r, mix(p.accent, p.bg, 0.35));
            let y = rect.center().y;
            painter.line_segment(
                [Pos2::new(rect.left() + 5.0, y), Pos2::new(rect.right() - 5.0, y)],
                Stroke::new(2.0_f32, p.accent_text),
            );
        }
    }
}

/// Disclosure triangle; `open` in 0..=1 animates the rotation.
pub fn paint_disclosure(painter: &egui::Painter, center: Pos2, open: f32, color: Color32) {
    let angle = open * std::f32::consts::FRAC_PI_2;
    let rot = egui::emath::Rot2::from_angle(angle);
    let pts = [Vec2::new(-3.0, -5.0), Vec2::new(-3.0, 5.0), Vec2::new(4.5, 0.0)]
        .map(|v| center + rot * v);
    painter.add(egui::Shape::convex_polygon(pts.to_vec(), color, Stroke::NONE));
}

/// Text laid out on one line and cut with an ellipsis when it does not fit.
pub fn truncated(ui: &Ui, text: &str, font: FontId, color: Color32, max_width: f32) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = egui::text::TextWrapping {
        max_width: max_width.max(10.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.fonts(|f| f.layout_job(job))
}

/// The app mark: three sediment layers.
pub fn paint_logo(painter: &egui::Painter, rect: Rect, p: &Palette) {
    let h = rect.height() / 3.0 - 2.0;
    let widths = [0.62, 0.84, 1.0];
    let colors = [p.accent, category_color(Category::Gradle), category_color(Category::Docker)];
    for i in 0..3 {
        let y = rect.top() + i as f32 * (h + 3.0);
        let w = rect.width() * widths[i];
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(w, h)),
            CornerRadius::same(3),
            colors[i],
        );
    }
}

pub struct StrataSegment {
    pub category: Category,
    pub found: u64,
    pub selected: u64,
}

/// Horizontal stacked bar: one segment per category, width = bytes found;
/// the solid part of each segment is what is currently marked for deletion.
/// Returns the category the user clicked, if any.
pub fn strata_bar(ui: &mut Ui, p: &Palette, segments: &[StrataSegment], height: f32) -> Option<Category> {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(7), p.raised);

    let total: u64 = segments.iter().map(|s| s.found).sum();
    if total == 0 {
        return None;
    }
    let gap = 2.0;
    let usable = rect.width() - gap * (segments.len().saturating_sub(1)) as f32;
    let mut x = rect.left();
    let mut clicked = None;
    for (i, s) in segments.iter().enumerate() {
        let w = (s.found as f32 / total as f32 * usable).max(3.0);
        let seg = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, rect.height()));
        let color = category_color(s.category);
        let cr = CornerRadius {
            nw: if i == 0 { 7 } else { 2 },
            sw: if i == 0 { 7 } else { 2 },
            ne: if i + 1 == segments.len() { 7 } else { 2 },
            se: if i + 1 == segments.len() { 7 } else { 2 },
        };
        painter.rect_filled(seg, cr, mix(color, p.raised, if p.dark { 0.68 } else { 0.62 }));
        if s.selected > 0 {
            let frac = (s.selected as f32 / s.found.max(1) as f32).clamp(0.0, 1.0);
            let mut solid = seg;
            solid.set_width(seg.width() * frac);
            painter.rect_filled(solid, cr, color);
        }
        let resp = ui.interact(seg, ui.id().with(("strata", i)), Sense::click());
        if resp.hovered() {
            painter.rect_stroke(seg, cr, Stroke::new(1.5_f32, p.text), StrokeKind::Inside);
        }
        let resp = resp.on_hover_ui(|ui| {
            ui.label(egui::RichText::new(crate::i18n::category_name(s.category)).font(strong(14.0)));
            ui.label(crate::i18n::strata_hover(
                &cleaner_core::format_bytes(s.found),
                &cleaner_core::format_bytes(s.selected),
            ));
            ui.label(egui::RichText::new(crate::i18n::click_to_filter()).color(p.muted).small());
        });
        if resp.clicked() {
            clicked = Some(s.category);
        }
        x += w + gap;
    }
    clicked
}

/// Decorative cross-section of sediment layers. `layers` are (colour, weight,
/// optional caption) from top to bottom; weights are normalised.
pub fn paint_landscape(ui: &Ui, rect: Rect, layers: &[(Color32, f32, Option<(String, String)>)], p: &Palette) {
    if layers.is_empty() {
        return;
    }
    let painter = ui.painter_at(rect);
    let total: f32 = layers.iter().map(|l| l.1).sum::<f32>().max(0.0001);
    let samples = 64;
    let top_pad = 26.0;
    let h = rect.height() - top_pad;

    // Boundary k is the top edge of layer k; boundary n is the flat bottom.
    let n = layers.len();
    let mut cum = vec![0.0f32; n + 1];
    for (i, l) in layers.iter().enumerate() {
        cum[i + 1] = cum[i] + l.1 / total;
    }
    let boundary = |k: usize, t: f32| -> f32 {
        let base = rect.top() + top_pad + cum[k] * h;
        if k == n {
            return rect.bottom();
        }
        // Amplitude shrinks with depth: older layers are flatter.
        let depth = k as f32 / n as f32;
        let amp = 12.0 * (1.0 - depth * 0.8);
        let phase = k as f32 * 1.7;
        base + amp * ((t * 5.2 + phase).sin() * 0.6 + (t * 11.0 + phase * 0.5).sin() * 0.25) - if k == 0 { 8.0 } else { 0.0 }
    };

    for (k, (color, _, _)) in layers.iter().enumerate() {
        let mut mesh = egui::Mesh::default();
        for i in 0..=samples {
            let t = i as f32 / samples as f32;
            let x = rect.left() + t * rect.width();
            let y0 = boundary(k, t);
            let y1 = boundary(k + 1, t).max(y0);
            mesh.colored_vertex(Pos2::new(x, y0), *color);
            mesh.colored_vertex(Pos2::new(x, y1), *color);
            if i > 0 {
                let b = (i as u32) * 2;
                mesh.add_triangle(b - 2, b - 1, b);
                mesh.add_triangle(b - 1, b, b + 1);
            }
        }
        painter.add(mesh);
    }
    // Thin separation lines between layers for a cut-rock look.
    for k in 1..n {
        let pts: Vec<Pos2> = (0..=samples)
            .map(|i| {
                let t = i as f32 / samples as f32;
                Pos2::new(rect.left() + t * rect.width(), boundary(k, t))
            })
            .collect();
        painter.add(egui::Shape::line(pts, Stroke::new(1.0_f32, with_alpha(p.bg, 120))));
    }
    // Captions on layers thick enough to hold them.
    for (k, (_, _, caption)) in layers.iter().enumerate() {
        let Some((name, size)) = caption else { continue };
        let t = 0.86;
        let y0 = boundary(k, t);
        let y1 = boundary(k + 1, t);
        if y1 - y0 < 20.0 {
            continue;
        }
        let ink = Color32::from_rgba_unmultiplied(15, 18, 24, 220);
        let pos = Pos2::new(rect.right() - 14.0, (y0 + y1) / 2.0);
        painter.text(pos, egui::Align2::RIGHT_CENTER, format!("{name}  {size}"), strong(13.0), ink);
    }
}

pub fn progress_bar(ui: &mut Ui, p: &Palette, fraction: f32, height: f32) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let r = CornerRadius::same((height / 2.0) as u8);
    ui.painter().rect_filled(rect, r, p.raised);
    let mut fill = rect;
    fill.set_width((rect.width() * fraction.clamp(0.0, 1.0)).max(height));
    ui.painter().rect_filled(fill, r, p.accent);
}

/// Card frame used for grouped content.
pub fn card(p: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(1.0_f32, p.line))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(18))
}

/// Lays `add` out in a column of at most `max_width`, centred horizontally.
pub fn centered_column<R>(ui: &mut Ui, max_width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let avail = ui.available_width();
    let w = avail.min(max_width);
    let pad = ((avail - w) / 2.0).max(0.0);
    ui.horizontal_top(|ui| {
        ui.add_space(pad);
        ui.vertical(|ui| {
            ui.set_width(w);
            add(ui)
        })
        .inner
    })
    .inner
}
