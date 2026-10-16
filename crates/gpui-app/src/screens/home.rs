//! Home: hero with the layered illustration, scan profiles, category tiles and
//! preferences.

use cleaner_core::{format_bytes, Category, Lang, ScanProfile};
use gpui::{
    canvas, div, point, prelude::*, px, Context, ElementId, FontWeight, Hsla, PathBuilder,
    SharedString, Window,
};

use crate::app::{DevCleaner, Screen};
use crate::i18n;
use crate::theme::{self, Palette};
use crate::widgets::{self, Kind};

const PAGE_X: f32 = 28.0;
const LANDSCAPE_H: f32 = 250.0;
const LANDSCAPE_TOP: f32 = 26.0;

pub fn render(app: &mut DevCleaner, window: &mut Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let width = f32::from(window.viewport_size().width) - 2.0 * PAGE_X;
    let wide = width > 760.0;

    let hero = div()
        .flex()
        .gap(px(28.))
        .child(hero_text(app, window, cx, if wide { (width * 0.46).min(480.0) } else { width }))
        .when(wide, |d| d.child(landscape(app, &p, window)));

    let scroll = div()
        .id("home-scroll")
        .size_full()
        .overflow_y_scroll()
        .track_scroll(&app.home_scroll)
        .child(
            div()
                .px(px(PAGE_X))
                .pt(px(14.))
                .pb(px(28.))
                .flex()
                .flex_col()
                .gap(px(30.))
                .child(hero)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(widgets::section_title(i18n::where_we_look()))
                        .child(tiles(app, width, window, cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(10.))
                        .child(widgets::section_title(i18n::preferences()))
                        .child(preferences(app, window, cx)),
                ),
        );
    div()
        .relative()
        .size_full()
        .child(scroll)
        .child(widgets::scrollbar(app, widgets::ScrollArea::Home, cx))
}

fn hero_text(app: &mut DevCleaner, window: &mut Window, cx: &mut Context<DevCleaner>, w: f32) -> impl IntoElement {
    let p = app.palette();
    let total = app.tree.total_found();
    let can_scan = !app.settings.scan_categories().is_empty();

    let mut actions = div().flex().items_center().gap(px(12.));
    if app.scanning {
        actions = actions.child(widgets::button(app, "hero-progress", i18n::view_scan_in_progress(), Kind::Primary, true, window, cx, |this, _, _| {
            this.screen = Screen::Results;
        }));
    } else if app.has_scanned && !app.tree.nodes.is_empty() {
        actions = actions
            .child(widgets::button(app, "hero-results", i18n::view_results(), Kind::Primary, true, window, cx, |this, _, _| {
                this.screen = Screen::Results;
            }))
            .child(widgets::button(app, "hero-rescan", i18n::scan_again(), Kind::Ghost, can_scan, window, cx, |this, _, _| this.start_scan()));
    } else {
        let btn = widgets::button(app, "hero-scan", i18n::scan_cta(), Kind::Primary, can_scan, window, cx, |this, _, _| this.start_scan());
        actions = actions.child(if can_scan { btn } else { btn.tooltip(widgets::simple_tooltip(p, i18n::empty_profile())) });
    }

    let mut profiles = div().flex().flex_wrap().gap(px(6.));
    for &prof in ScanProfile::all() {
        let on = app.settings.profile == prof;
        profiles = profiles.child(
            widgets::chip(app, SharedString::from(format!("profile-{prof:?}")), i18n::profile_name(prof), on, None, window, cx, move |this, _, _| {
                this.settings.profile = prof;
                this.save_settings();
            })
            .tooltip(widgets::simple_tooltip(p, i18n::profile_hint(prof))),
        );
    }

    div()
        .w(px(w))
        .flex_none()
        .flex()
        .flex_col()
        .pt(px(18.))
        .child(
            div()
                .font_family(theme::display_font())
                .text_size(px(36.))
                .line_height(px(42.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(i18n::hero_title()),
        )
        .child(
            div()
                .mt(px(12.))
                .text_size(px(15.))
                .line_height(px(22.))
                .text_color(p.muted)
                .child(i18n::hero_body()),
        )
        .child(
            div()
                .mt(px(20.))
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.muted)
                .child(i18n::scan_profile()),
        )
        .child(div().mt(px(8.)).child(profiles))
        .child(
            div()
                .mt(px(6.))
                .text_size(px(12.))
                .text_color(p.muted)
                .child(i18n::profile_hint(app.settings.profile)),
        )
        .child(div().mt(px(20.)).child(actions))
        .when(app.has_scanned && total > 0, |d| {
            d.child(
                div()
                    .mt(px(16.))
                    .flex()
                    .items_baseline()
                    .gap(px(8.))
                    .child(
                        div()
                            .font_family(theme::display_font())
                            .text_size(px(24.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.accent_ink)
                            .child(format_bytes(total)),
                    )
                    .child(div().text_color(p.muted).child(i18n::found_summary(app.tree.root_nodes().len()))),
            )
        })
}

/// Layers for the illustration: real proportions after a scan (heaviest at
/// the bottom, like the oldest sediment), a quiet placeholder before.
fn layers(app: &DevCleaner, p: &Palette) -> Vec<(Hsla, f32, Option<(String, String)>)> {
    let segs = app.strata_segments();
    if segs.is_empty() {
        let fade = if p.dark { 0.5 } else { 0.4 };
        return [
            (Category::Temp, 0.7),
            (Category::Yarn, 0.9),
            (Category::Gradle, 1.2),
            (Category::Cargo, 1.0),
            (Category::Docker, 1.6),
            (Category::Maven, 1.1),
        ]
        .into_iter()
        .map(|(c, w)| (theme::mix(theme::category_color(c), p.bg, fade), w, None))
        .collect();
    }
    let total: u64 = segs.iter().map(|s| s.1).sum();
    let mut out: Vec<_> = segs
        .iter()
        .take(8)
        .map(|(c, found, _)| {
            let w = (*found as f32 / total.max(1) as f32).max(0.06);
            (
                theme::category_color(*c),
                w,
                Some((i18n::category_name(*c).to_string(), format_bytes(*found))),
            )
        })
        .collect();
    out.reverse();
    out
}

/// y of boundary `k` (top edge of layer k) at horizontal position t in 0..1.
fn boundary(cum: &[f32], k: usize, t: f32) -> f32 {
    let n = cum.len() - 1;
    let h = LANDSCAPE_H - LANDSCAPE_TOP;
    if k == n {
        return LANDSCAPE_H;
    }
    let base = LANDSCAPE_TOP + cum[k] * h;
    let depth = k as f32 / n as f32;
    let amp = 12.0 * (1.0 - depth * 0.8);
    let phase = k as f32 * 1.7;
    base + amp * ((t * 5.2 + phase).sin() * 0.6 + (t * 11.0 + phase * 0.5).sin() * 0.25)
        - if k == 0 { 8.0 } else { 0.0 }
}

/// Intro: layers rise from the bottom one after another, deepest first.
const INTRO_RISE: f32 = 0.75;
const INTRO_STAGGER: f32 = 0.09;

fn ease_out_cubic(x: f32) -> f32 {
    1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3)
}

/// Rise progress of boundary `k` (0 = flat at the bottom, 1 = in place).
fn rise(elapsed: f32, k: usize, n: usize) -> f32 {
    let delay = (n.saturating_sub(1 + k)) as f32 * INTRO_STAGGER;
    ease_out_cubic((elapsed - delay) / INTRO_RISE)
}

fn landscape(app: &mut DevCleaner, p: &Palette, window: &mut Window) -> impl IntoElement {
    let started = *app.intro_at.get_or_insert_with(std::time::Instant::now);
    let elapsed = started.elapsed().as_secs_f32();
    let layers = layers(app, p);
    let intro_total = INTRO_RISE + layers.len() as f32 * INTRO_STAGGER + 0.35;
    if elapsed < intro_total {
        window.request_animation_frame();
    }
    let n_layers = layers.len();
    // Boundary k animated: starts at the bottom edge and rises into place.
    // A layer that has not risen yet rests on the one below it (zero
    // thickness), so boundaries never cross while they move.
    let animated = move |cum: &[f32], k: usize, t: f32| {
        let mut y = LANDSCAPE_H;
        for j in (k..n_layers).rev() {
            let target = boundary(cum, j, t);
            let risen = LANDSCAPE_H - (LANDSCAPE_H - target) * rise(elapsed, j, n_layers);
            y = risen.min(y);
        }
        y
    };
    let total: f32 = layers.iter().map(|l| l.1).sum::<f32>().max(0.0001);
    let mut cum = vec![0.0f32; layers.len() + 1];
    for (i, l) in layers.iter().enumerate() {
        cum[i + 1] = cum[i] + l.1 / total;
    }
    let colors: Vec<Hsla> = layers.iter().map(|l| l.0).collect();
    let seam = theme::alpha(p.bg, 0.45);
    let cum_paint = cum.clone();

    let drawing = canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let samples = 64;
            let x0 = bounds.origin.x;
            let y0 = bounds.origin.y;
            let w = f32::from(bounds.size.width);
            for (k, color) in colors.iter().enumerate() {
                let mut path = PathBuilder::fill();
                for i in 0..=samples {
                    let t = i as f32 / samples as f32;
                    let pt = point(x0 + px(t * w), y0 + px(animated(&cum_paint, k, t)));
                    if i == 0 { path.move_to(pt) } else { path.line_to(pt) }
                }
                for i in (0..=samples).rev() {
                    let t = i as f32 / samples as f32;
                    let y = animated(&cum_paint, k + 1, t).max(animated(&cum_paint, k, t));
                    path.line_to(point(x0 + px(t * w), y0 + px(y)));
                }
                path.close();
                if let Ok(path) = path.build() {
                    window.paint_path(path, *color);
                }
            }
            // Thin seams between layers for a cut-rock look.
            for k in 1..colors.len() {
                let mut path = PathBuilder::stroke(px(1.));
                for i in 0..=samples {
                    let t = i as f32 / samples as f32;
                    let pt = point(x0 + px(t * w), y0 + px(animated(&cum_paint, k, t)));
                    if i == 0 { path.move_to(pt) } else { path.line_to(pt) }
                }
                if let Ok(path) = path.build() {
                    window.paint_path(path, seam);
                }
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();

    // Captions on layers thick enough to hold them (positions only depend on
    // the fixed height, so they are laid out as normal elements).
    let caption_fade = ((elapsed - intro_total + 0.45) / 0.3).clamp(0.0, 1.0);
    let ink = gpui::rgba(0x0F1218DD);
    let captions = layers.iter().enumerate().filter_map(|(k, l)| {
        let (name, size) = l.2.clone()?;
        let y0 = boundary(&cum, k, 0.86);
        let y1 = boundary(&cum, k + 1, 0.86);
        (y1 - y0 >= 20.0).then(|| {
            div()
                .absolute()
                .right(px(14.))
                .top(px((y0 + y1) / 2.0 - 9.0))
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(ink)
                .opacity(caption_fade)
                .child(format!("{name}  {size}"))
        })
    });

    div()
        .relative()
        .flex_1()
        .h(px(LANDSCAPE_H))
        .rounded_b(px(14.))
        .overflow_hidden()
        .child(drawing)
        .children(captions)
}

fn tiles(app: &mut DevCleaner, width: f32, window: &mut Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let gap = 10.0;
    let cols = ((width + gap) / (190.0 + gap)).floor().max(1.0);
    let tile_w = ((width - gap * (cols - 1.0)) / cols).floor() - 0.5;

    let mut cats: Vec<Category> = Category::all().to_vec();
    if app.has_scanned {
        let found = app.tree.found_by_category();
        cats.sort_by(|a, b| found.get(b).copied().unwrap_or(0).cmp(&found.get(a).copied().unwrap_or(0)));
    }
    let in_profile = app.settings.scan_categories();
    let custom = app.settings.profile == ScanProfile::Custom;

    let items: Vec<_> = cats
        .into_iter()
        .map(|cat| {
            let status = app.tree.category_status.get(&cat).cloned();
            let available = status.as_ref().is_some_and(|s| s.available);
            let included = in_profile.contains(&cat);
            let clickable = custom || (available && included);
            let color = theme::category_color(cat);
            let id: SharedString = format!("tile-{cat:?}").into();
            let t = app.hover_t(&id, window);

            let (value, big, value_color) = match &status {
                _ if !included => (i18n::outside_profile().to_string(), false, p.muted),
                None if app.scanning => (i18n::tile_searching().to_string(), false, p.muted),
                None => (i18n::tile_not_checked().to_string(), false, p.muted),
                Some(s) if s.available && s.total_bytes == 0 && s.disk_bytes > 0 => {
                    (i18n::disk_size(&format_bytes(s.disk_bytes)), false, p.text)
                }
                Some(s) if s.available => (format_bytes(s.total_bytes), true, p.text),
                Some(_) => (i18n::tile_nothing().to_string(), false, p.muted),
            };
            let faded = !included || status.as_ref().is_some_and(|s| !s.available);
            let border = if custom && included { p.accent_ink } else { p.line };
            let lift = if clickable { t } else { 0.0 };

            let tile = div()
                .id(ElementId::Name(id.clone()))
                .relative()
                .w(px(tile_w))
                .h(px(84.))
                .rounded(px(14.))
                .border_1()
                .border_color(border)
                .bg(if included { theme::mix(p.surface, p.raised, lift) } else { p.bg })
                .when(clickable, |d| d.cursor_pointer())
                .child(
                    div()
                        .absolute()
                        .left(px(14.))
                        .top(px(16.))
                        .bottom(px(16.))
                        .w(px(4.))
                        .rounded_full()
                        .bg(if faded { theme::mix(color, p.surface, 0.7) } else { color }),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(30.))
                        .right(px(14.))
                        .top(px(14.))
                        .bottom(px(13.))
                        .flex()
                        .flex_col()
                        .justify_between()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(if included { p.text } else { p.muted })
                                .truncate()
                                .child(i18n::category_name(cat)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_baseline()
                                .justify_between()
                                .gap(px(6.))
                                .child(
                                    div()
                                        .truncate()
                                        .text_color(value_color)
                                        .when(big, |d| {
                                            d.font_family(theme::display_font())
                                                .text_size(px(20.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                        })
                                        .when(!big, |d| d.text_size(px(13.)))
                                        .child(value),
                                )
                                .when_some(status.as_ref().filter(|s| s.available && included), |d, s| {
                                    d.child(
                                        div()
                                            .flex_none()
                                            .text_size(px(12.))
                                            .text_color(p.muted)
                                            .child(i18n::items(s.item_count)),
                                    )
                                }),
                        ),
                )
                .on_hover(cx.listener({
                    let id = id.clone();
                    move |this, h: &bool, _, cx| {
                        this.set_hover(&id, *h);
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if custom {
                        let list = &mut this.settings.custom_categories;
                        if let Some(i) = list.iter().position(|c| *c == cat) {
                            list.remove(i);
                        } else {
                            list.push(cat);
                        }
                        this.save_settings();
                    } else if available && included {
                        this.filter.categories.clear();
                        this.filter.categories.insert(cat);
                        this.screen = Screen::Results;
                    }
                    cx.notify();
                }));

            let hint: Option<String> = if custom {
                Some(if included { i18n::tile_toggle_exclude() } else { i18n::tile_toggle_include() }.into())
            } else if let Some(r) = status.as_ref().and_then(|s| s.unavailable_reason.as_ref()) {
                Some(r.to_string())
            } else if available && included {
                Some(i18n::tile_hover().into())
            } else {
                None
            };
            match hint {
                Some(h) => tile.tooltip(widgets::simple_tooltip(p, h)),
                None => tile,
            }
        })
        .collect();

    div().flex().flex_wrap().gap(px(gap)).children(items)
}

fn preferences(app: &mut DevCleaner, window: &mut Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let row = |title: SharedString, body: SharedString, control: gpui::Div| {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(20.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(div().text_size(px(12.)).text_color(p.muted).child(body)),
            )
            .child(control)
    };

    let mut ages = div().flex().flex_wrap().justify_end().gap(px(6.));
    for days in [0u32, 1, 3, 7, 14, 30, 60, 90] {
        let on = app.settings.temp_min_age_days == days;
        ages = ages.child(widgets::chip(app, SharedString::from(format!("age-{days}")), format!("{days}{}", i18n::days_suffix()), on, None, window, cx, move |this, _, _| {
            this.settings.temp_min_age_days = days;
            this.save_settings();
        }));
    }

    let current = cleaner_core::i18n::lang();
    let mut langs = div().flex().gap(px(6.));
    for &l in Lang::all() {
        langs = langs.child(widgets::chip(app, SharedString::from(format!("pref-lang-{}", l.code())), l.native_name(), current == l, None, window, cx, move |this, _, _| {
            this.set_language(l)
        }));
    }

    widgets::card(&p)
        .flex()
        .flex_col()
        .gap(px(14.))
        .child(row(i18n::temp_age_title().into(), i18n::temp_age_body().into(), ages))
        .child(div().h(px(1.)).bg(p.line))
        .child(row(i18n::language().into(), i18n::language_body().into(), langs))
}
