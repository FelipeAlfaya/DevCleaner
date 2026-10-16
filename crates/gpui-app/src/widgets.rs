//! Building blocks: buttons, chips, checkbox, pills, menus, tooltips and the
//! top bar. Everything is drawn with plain GPUI elements.

use cleaner_core::{CheckState, Lang};
use gpui::{
    anchored, canvas, deferred, div, point, prelude::*, px, relative, AnyView, App, ClickEvent,
    Context, Div, FontWeight, Hsla, MouseButton, MouseDownEvent, PathBuilder, ScrollHandle,
    SharedString, Stateful, Window,
};

use crate::app::{DevCleaner, Menu, Screen};
use crate::i18n;
use crate::theme::{self, Palette};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Danger,
    Ghost,
}

type Handler = dyn Fn(&mut DevCleaner, &mut Window, &mut Context<DevCleaner>);

// Boxed so the listener type does not borrow `cx`.
fn hover_listener(id: SharedString, cx: &mut Context<DevCleaner>) -> Box<dyn Fn(&bool, &mut Window, &mut App)> {
    Box::new(cx.listener(move |this, hovered: &bool, _, cx| {
        this.set_hover(&id, *hovered);
        cx.notify();
    }))
}

fn click_listener(
    on_click: impl Fn(&mut DevCleaner, &mut Window, &mut Context<DevCleaner>) + 'static,
    cx: &mut Context<DevCleaner>,
) -> Box<dyn Fn(&ClickEvent, &mut Window, &mut App)> {
    let on_click: Box<Handler> = Box::new(on_click);
    Box::new(cx.listener(move |this, _: &ClickEvent, window, cx| {
        on_click(this, window, cx);
        cx.notify();
    }))
}

/// Opacity while hovered; buttons fade towards it in 0.2s.
const HOVER_OPACITY: f32 = 0.82;

fn hover_opacity(t: f32) -> f32 {
    1.0 - (1.0 - HOVER_OPACITY) * t
}

/// Main action button. Fades slightly while hovered.
#[allow(clippy::too_many_arguments)]
pub fn button(
    app: &DevCleaner,
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    kind: Kind,
    enabled: bool,
    window: &Window,
    cx: &mut Context<DevCleaner>,
    on_click: impl Fn(&mut DevCleaner, &mut Window, &mut Context<DevCleaner>) + 'static,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let p = app.palette();
    let t = if enabled { app.hover_t(&id, window) } else { 0.0 };
    let (bg, fg) = match kind {
        Kind::Primary => (p.accent, p.accent_text),
        Kind::Danger => (p.danger, gpui::white()),
        Kind::Ghost => (if p.dark { p.bg } else { p.surface }, p.text),
    };
    let el = div()
        .id(id.clone())
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(px(40.))
        .px(px(22.))
        .rounded(px(10.))
        .bg(bg)
        .text_color(fg)
        .text_size(px(15.))
        .font_weight(if kind == Kind::Ghost { FontWeight::NORMAL } else { FontWeight::SEMIBOLD })
        .whitespace_nowrap()
        .when(kind == Kind::Ghost, |d| d.border_1().border_color(p.line))
        .child(label.into());
    if enabled {
        el.opacity(hover_opacity(t))
            .cursor_pointer()
            .on_hover(hover_listener(id, cx))
            .on_click(click_listener(on_click, cx))
    } else {
        el.opacity(0.45)
    }
}

/// Small rounded filter/toggle chip. Turns yellow while hovered, like the
/// selected one.
#[allow(clippy::too_many_arguments)]
pub fn chip(
    app: &DevCleaner,
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    selected: bool,
    dot: Option<Hsla>,
    window: &Window,
    cx: &mut Context<DevCleaner>,
    on_click: impl Fn(&mut DevCleaner, &mut Window, &mut Context<DevCleaner>) + 'static,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let p = app.palette();
    let t = app.hover_t(&id, window);
    // Hover blends an idle chip towards the selected (yellow) look in 0.2s.
    let on_bg = theme::mix(p.accent, p.bg, if p.dark { 0.8 } else { 0.72 });
    let (bg, border, fg) = if selected {
        (on_bg, p.accent_ink, p.text)
    } else {
        (theme::mix(p.bg, on_bg, t), theme::mix(p.line, p.accent_ink, t), theme::mix(p.muted, p.text, t))
    };
    div()
        .id(id.clone())
        .flex()
        .flex_none()
        .items_center()
        .gap(px(7.))
        .h(px(28.))
        .px(px(12.))
        .rounded_full()
        .border_1()
        .border_color(border)
        .bg(bg)
        .text_color(fg)
        .text_size(px(13.))
        .whitespace_nowrap()
        .cursor_pointer()
        .when_some(dot, |d, c| d.child(div().size(px(8.)).rounded_full().bg(c)))
        .child(label.into())
        .on_hover(hover_listener(id, cx))
        .on_click(click_listener(on_click, cx))
}

pub fn checkbox(p: &Palette, state: CheckState, enabled: bool) -> Div {
    let base = div().size(px(18.)).flex_none().rounded(px(5.)).flex().items_center().justify_center();
    if !enabled {
        return base
            .bg(p.raised)
            .border_1()
            .border_color(p.line)
            .child(div().w(px(8.)).h(px(6.)).rounded(px(1.)).bg(p.muted));
    }
    match state {
        CheckState::Unchecked => base.border(px(1.5)).border_color(p.muted),
        CheckState::Checked => base
            .bg(p.accent)
            .text_color(p.accent_text)
            .text_size(px(13.))
            .font_weight(FontWeight::BOLD)
            .child("✓"),
        CheckState::Partial => base
            .bg(theme::mix(p.accent, p.bg, 0.35))
            .child(div().w(px(8.)).h(px(2.)).rounded_full().bg(p.accent_text)),
    }
}

pub fn pill(p: &Palette, text: impl Into<SharedString>, color: Hsla) -> Div {
    div()
        .flex_none()
        .px(px(9.))
        .py(px(2.))
        .rounded_full()
        .bg(p.tint(color))
        .text_color(color)
        .text_size(px(11.5))
        .whitespace_nowrap()
        .child(text.into())
}

pub fn progress_bar(p: &Palette, fraction: f32, height: f32) -> Div {
    div()
        .w_full()
        .h(px(height))
        .rounded_full()
        .bg(p.raised)
        .child(
            div()
                .h_full()
                .min_w(px(height))
                .w(relative(fraction.clamp(0.0, 1.0)))
                .rounded_full()
                .bg(p.accent),
        )
}

pub fn card(p: &Palette) -> Div {
    div()
        .bg(p.surface)
        .border_1()
        .border_color(p.line)
        .rounded(px(14.))
        .p(px(18.))
        .shadow_sm()
}

pub fn note(p: &Palette, color: Hsla, text: impl Into<SharedString>) -> Div {
    div()
        .w_full()
        .p(px(12.))
        .rounded(px(10.))
        .bg(theme::mix(color, p.bg, 0.87))
        .border_1()
        .border_color(theme::mix(color, p.bg, 0.5))
        .text_color(color)
        .child(text.into())
}

pub fn section_title(text: impl Into<SharedString>) -> Div {
    div().text_size(px(16.)).font_weight(FontWeight::SEMIBOLD).child(text.into())
}

/// Disclosure triangle, rotated when open.
pub fn disclosure(open: bool, color: Hsla) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let c = bounds.center();
            let (a, b, d) = if open {
                (point(c.x - px(5.), c.y - px(3.)), point(c.x + px(5.), c.y - px(3.)), point(c.x, c.y + px(4.)))
            } else {
                (point(c.x - px(3.), c.y - px(5.)), point(c.x - px(3.), c.y + px(5.)), point(c.x + px(4.), c.y))
            };
            let mut path = PathBuilder::fill();
            path.move_to(a);
            path.line_to(b);
            path.line_to(d);
            path.close();
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(20.))
}

/// Indeterminate spinner: a faint ring with an arc that rotates while its
/// length breathes in and out (Material-style).
pub fn spinner(color: Hsla, size: f32) -> impl IntoElement {
    use std::f32::consts::TAU;
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    let start = *START.get_or_init(Instant::now);

    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let t = start.elapsed().as_secs_f32();
            let stroke = (size / 8.0).max(2.0);
            let c = bounds.center();
            let r = size / 2.0 - stroke / 2.0;
            let at = |a: f32| point(c.x + px(r * a.cos()), c.y + px(r * a.sin()));

            let arc = |from: f32, sweep: f32| {
                let steps = ((sweep.abs() / TAU) * 48.0).ceil().max(2.0) as usize;
                let mut path = PathBuilder::stroke(px(stroke));
                path.move_to(at(from));
                for i in 1..=steps {
                    path.line_to(at(from + sweep * i as f32 / steps as f32));
                }
                path.build().ok()
            };

            // Track.
            if let Some(path) = arc(0.0, TAU * 0.999) {
                window.paint_path(path, theme::alpha(color, 0.18));
            }
            // Head and tail chase each other: the arc grows to ~270° and
            // shrinks back to ~20° every 1.4s, while the whole thing spins.
            let cycle = (t / 1.4).fract();
            let ease = |x: f32| x * x * (3.0 - 2.0 * x);
            let (head, tail) = if cycle < 0.5 {
                (ease(cycle * 2.0), 0.0)
            } else {
                (1.0, ease((cycle - 0.5) * 2.0))
            };
            let min = 0.06 * TAU;
            let max = 0.75 * TAU;
            let lap = (t / 1.4).floor() * (max - min);
            let base = t * TAU / 1.6 - TAU / 4.0 + lap;
            let from = base + tail * (max - min);
            let sweep = min + (head - tail) * (max - min);
            if let Some(path) = arc(from, sweep) {
                window.paint_path(path, color);
            }
            window.request_animation_frame();
        },
    )
    .flex_none()
    .size(px(size))
}

// ---------------------------------------------------------------- tooltip

pub struct Tip {
    p: Palette,
    lines: Vec<(SharedString, Option<Hsla>, bool)>,
}

impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let p = self.p;
        // Text only wraps correctly inside a definite width.
        let longest = self.lines.iter().map(|l| l.0.chars().count()).max().unwrap_or(0);
        div()
            .font_family(theme::body_font())
            .when(longest > 44, |d| d.w(px(360.)))
            .px(px(12.))
            .py(px(9.))
            .rounded(px(9.))
            .bg(p.surface)
            .border_1()
            .border_color(p.line)
            .shadow_lg()
            .text_size(px(13.))
            .text_color(p.text)
            .flex()
            .flex_col()
            .gap(px(4.))
            .children(self.lines.iter().map(|(t, c, bold)| {
                div()
                    .w_full()
                    .when_some(*c, |d, c| d.text_color(c))
                    .when(*bold, |d| d.font_weight(FontWeight::SEMIBOLD))
                    .child(t.clone())
            }))
    }
}

/// Tooltip builder for `.tooltip(...)`. Each line is (text, colour, bold).
pub fn tooltip(
    p: Palette,
    lines: Vec<(SharedString, Option<Hsla>, bool)>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |_, cx| cx.new(|_| Tip { p, lines: lines.clone() }).into()
}

pub fn simple_tooltip(p: Palette, text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    tooltip(p, vec![(text.into(), None, false)])
}

// ---------------------------------------------------------------- menus

/// A trigger that opens a small list of options below it.
#[allow(clippy::too_many_arguments)]
pub fn dropdown<T: Copy + PartialEq + 'static>(
    app: &DevCleaner,
    menu: Menu,
    id: &'static str,
    label: impl Into<SharedString>,
    width: f32,
    options: Vec<(T, SharedString)>,
    current: T,
    window: &Window,
    cx: &mut Context<DevCleaner>,
    on_pick: impl Fn(&mut DevCleaner, T) + 'static,
) -> Div {
    let p = app.palette();
    let open = app.menu == Some(menu);
    let t = app.hover_t(id, window);
    let trigger = div()
        .id(id)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.))
        .w(px(width))
        .h(px(30.))
        .px(px(12.))
        .rounded(px(8.))
        .bg(if t > 0.01 || open { p.raised } else { p.surface })
        .border_1()
        .border_color(if open { p.accent_ink } else { p.line })
        .text_size(px(13.5))
        .cursor_pointer()
        .child(div().truncate().child(label.into()))
        .child(div().text_color(p.muted).text_size(px(10.)).child(if open { "▲" } else { "▼" }))
        .on_hover(hover_listener(id.into(), cx))
        .on_click(click_listener(
            move |this, _, _| {
                this.menu = if this.menu == Some(menu) { None } else { Some(menu) };
            },
            cx,
        ));

    let on_pick = std::rc::Rc::new(on_pick);
    let list = open.then(|| {
        let items: Vec<_> = options
            .into_iter()
            .enumerate()
            .map(|(i, (value, text))| {
                let on_pick = on_pick.clone();
                let selected = value == current;
                div()
                    .id(SharedString::from(format!("{id}-opt-{i}")))
                    .px(px(10.))
                    .py(px(7.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .text_size(px(13.5))
                    .when(selected, |d| d.bg(theme::mix(p.accent, p.surface, 0.78)).text_color(p.text))
                    .hover(|s| s.bg(p.raised))
                    .child(text)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        on_pick(this, value);
                        this.menu = None;
                        cx.notify();
                    }))
            })
            .collect();
        deferred(
            anchored().child(
                div()
                    .id(SharedString::from(format!("{id}-list")))
                    .occlude()
                    .mt(px(4.))
                    .min_w(px(width))
                    .p(px(4.))
                    .rounded(px(10.))
                    .bg(p.surface)
                    .border_1()
                    .border_color(p.line)
                    .shadow_lg()
                    .text_color(p.text)
                    .font_family(theme::body_font())
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .children(items),
            ),
        )
        .with_priority(1)
    });

    div().flex().flex_col().child(trigger).children(list)
}

/// Transparent layer that closes the open menu when clicking anywhere else.
pub fn menu_backdrop(cx: &mut Context<DevCleaner>) -> impl IntoElement {
    deferred(
        div()
            .id("menu-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.menu = None;
                    cx.notify();
                }),
            ),
    )
}

// ---------------------------------------------------------------- top bar

pub fn logo(p: &Palette, size: f32) -> Div {
    let h = size / 3.0 - 2.0;
    div()
        .flex()
        .flex_col()
        .gap(px(3.))
        .w(px(size))
        .children([(0.62, p.accent), (0.84, theme::category_color(cleaner_core::Category::Gradle)), (1.0, theme::category_color(cleaner_core::Category::Docker))].map(|(w, c)| {
            div().h(px(h)).w(relative(w)).rounded(px(3.)).bg(c)
        }))
}

pub fn top_bar(app: &mut DevCleaner, window: &mut Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let in_results = matches!(app.screen, Screen::Results | Screen::Confirm);
    let has_results = !app.tree.nodes.is_empty() || app.scanning;
    let lang = cleaner_core::i18n::lang();

    let mut nav = div().flex().items_center().gap(px(8.));
    if !app.cleaning {
        nav = nav.child(chip(app, "nav-home", i18n::nav_home(), app.screen == Screen::Home, None, window, cx, |this, _, _| this.go_home()));
        if has_results {
            nav = nav.child(chip(app, "nav-results", i18n::nav_results(), in_results, None, window, cx, |this, _, _| {
                this.screen = Screen::Results;
            }));
        }
    }

    let mut right = div().flex().items_center().gap(px(10.));
    if app.scanning && app.screen != Screen::Results {
        right = right.child(
            div()
                .text_color(p.muted)
                .text_size(px(13.))
                .child(i18n::scanning_short(app.categories_done, app.scan_total)),
        );
    }
    right = right
        .child(dropdown(
            app,
            Menu::Language,
            "lang-menu",
            lang.code().to_uppercase(),
            72.0,
            Lang::all().iter().map(|l| (*l, SharedString::from(l.native_name()))).collect(),
            lang,
            window,
            cx,
            |this, l| this.set_language(l),
        ))
        .child(chip(app, "theme", i18n::theme_toggle(app.settings.dark_theme), false, None, window, cx, |this, _, _| {
            this.settings.dark_theme = !this.settings.dark_theme;
            this.save_settings();
        }));

    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_between()
        .px(px(28.))
        .py(px(14.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(logo(&p, 24.0))
                .child(
                    div()
                        .font_family(theme::display_font())
                        .text_size(px(20.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(i18n::app_title()),
                )
                .child(div().w(px(16.)))
                .child(nav),
        )
        .child(right)
}

// ---------------------------------------------------------------- scrollbar

/// Scrollable areas that get a visible scrollbar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScrollArea {
    Home,
    Results,
    Confirm,
    Report,
}

/// Geometry of a scrollbar thumb from the last layout of `handle`:
/// (viewport top, viewport height, thumb top offset, thumb height, max scroll).
pub fn thumb_geometry(handle: &ScrollHandle) -> Option<(f32, f32, f32, f32, f32)> {
    let bounds = handle.bounds();
    let view = f32::from(bounds.size.height);
    let max = f32::from(handle.max_offset().height);
    if view <= 0.0 || max <= 0.5 {
        return None;
    }
    let thumb = (view * view / (view + max)).clamp(32.0, view);
    let scrolled = (-f32::from(handle.offset().y)).clamp(0.0, max);
    let top = scrolled / max * (view - thumb);
    Some((f32::from(bounds.origin.y), view, top, thumb, max))
}

/// Thin overlay scrollbar for `area`. The thumb can be dragged and the
/// track clicked to jump; the wheel and touchpad keep working as usual.
pub fn scrollbar(app: &DevCleaner, area: ScrollArea, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let handle = app.scroll_handle(area);
    let Some((_, _, top, thumb, _)) = thumb_geometry(&handle) else {
        return div().into_any_element();
    };
    let dragging = app.scroll_drag.is_some_and(|d| d.0 == area);
    let thumb_color = theme::alpha(p.muted, if dragging { 0.75 } else { 0.4 });
    div()
        .id(SharedString::from(format!("scroll-track-{area:?}")))
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .w(px(14.))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                let handle = this.scroll_handle(area);
                if let Some((view_top, view, _, thumb, max)) = thumb_geometry(&handle) {
                    // Jump so the thumb centres on the click, then keep dragging.
                    let y = f32::from(ev.position.y) - view_top - thumb / 2.0;
                    let target = (y / (view - thumb)).clamp(0.0, 1.0) * max;
                    handle.set_offset(point(handle.offset().x, px(-target)));
                    this.scroll_drag = Some((area, f32::from(ev.position.y), target));
                    cx.stop_propagation();
                    cx.notify();
                }
            }),
        )
        .child(
            div()
                .id(SharedString::from(format!("scroll-thumb-{area:?}")))
                .absolute()
                .right(px(3.))
                .top(px(top))
                .w(px(if dragging { 8. } else { 6. }))
                .h(px(thumb))
                .rounded_full()
                .bg(thumb_color)
                .hover(|s| s.bg(theme::alpha(p.muted, 0.7)).w(px(8.)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                        let handle = this.scroll_handle(area);
                        let current = -f32::from(handle.offset().y);
                        this.scroll_drag = Some((area, f32::from(ev.position.y), current));
                        cx.stop_propagation();
                        cx.notify();
                    }),
                ),
        )
        .into_any_element()
}
