//! Confirm, cleaning (with the compaction view) and report screens.

use std::time::Instant;

use cleaner_core::{compact_estimate_all, format_bytes, Category, CompactProgress, DeleteOutcome, Risk, ScanNode};
use gpui::{div, prelude::*, px, relative, ClickEvent, Context, Div, FontWeight, Hsla, SharedString, Window};

use crate::app::{DevCleaner, Screen};
use crate::i18n;
use crate::theme::{self, Palette};
use crate::widgets::{self, Kind, ScrollArea};

fn column(max_w: f32) -> Div {
    div().w_full().max_w(px(max_w)).flex().flex_col()
}

fn page(app: &DevCleaner, area: ScrollArea, inner: Div, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let handle = app.scroll_handle(area);
    div()
        .relative()
        .size_full()
        .child(
            div()
                .id(SharedString::from(format!("page-{area:?}")))
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&handle)
                .child(div().w_full().flex().justify_center().px(px(28.)).py(px(24.)).child(inner)),
        )
        .child(widgets::scrollbar(app, area, cx))
}

fn big(text: impl Into<SharedString>, size: f32, color: Hsla) -> Div {
    div()
        .font_family(theme::display_font())
        .text_size(px(size))
        .line_height(px(size * 1.15))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color)
        .child(text.into())
}

fn dot(c: Hsla) -> Div {
    div().size(px(9.)).flex_none().rounded_full().bg(c)
}

// ---------------------------------------------------------------- confirm

pub fn confirm(app: &mut DevCleaner, window: &mut Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let targets = app.tree.targets();
    let (count, bytes) = app.tree.selected_summary();
    let mut by_cat: Vec<(Category, u64)> = app.tree.selected_by_category().into_iter().collect();
    by_cat.sort_by(|a, b| b.1.cmp(&a.1));
    let has_review = targets
        .iter()
        .filter_map(|id| app.tree.node(*id))
        .any(|n| n.risk == Risk::Review && !n.is_vhd());
    let disks: Vec<ScanNode> = app
        .tree
        .selected_vhds()
        .iter()
        .filter_map(|id| app.tree.node(*id).cloned())
        .collect();
    let mut items: Vec<ScanNode> = targets.iter().filter_map(|id| app.tree.node(*id).cloned()).collect();
    items.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    let mut col = column(660.).gap(px(14.)).child(big(i18n::confirm_title(), 30., p.text)).child(
        div().text_color(p.muted).child(i18n::confirm_body(count, &format_bytes(bytes))),
    );

    if bytes > 0 || disks.is_empty() {
        let max = by_cat.first().map(|c| c.1).unwrap_or(1).max(1);
        col = col.child(
            widgets::card(&p)
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(div().text_size(px(12.)).text_color(p.muted).child(i18n::space_to_free()))
                .child(big(format_bytes(bytes), 46., p.accent_ink))
                .children(by_cat.iter().map(|(cat, b)| {
                    let c = theme::category_color(*cat);
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(dot(c))
                        .child(div().w(px(132.)).flex_none().truncate().child(i18n::category_name(*cat)))
                        .child(
                            div()
                                .flex_1()
                                .h(px(8.))
                                .rounded_full()
                                .bg(p.raised)
                                .child(div().h_full().min_w(px(4.)).w(relative(*b as f32 / max as f32)).rounded_full().bg(c)),
                        )
                        .child(
                            div()
                                .w(px(90.))
                                .flex()
                                .justify_end()
                                .font_family(theme::display_font())
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format_bytes(*b)),
                        )
                })),
        );
    }

    if !disks.is_empty() {
        let (fast, slow) = compact_estimate_all(disks.iter().map(|d| d.size_bytes));
        col = col.child(
            widgets::card(&p)
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(widgets::section_title(i18n::disks_to_compact()))
                .children(disks.iter().map(|d| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(dot(theme::category_color(d.category)))
                        .child(div().flex_1().min_w_0().truncate().child(d.label.to_string()))
                        .child(div().text_color(p.muted).child(format_bytes(d.size_bytes)))
                }))
                .child(div().mt(px(4.)).font_weight(FontWeight::SEMIBOLD).child(i18n::compact_estimate(fast, slow)))
                .child(
                    widgets::note(&p, p.review, i18n::compact_slow_note())
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(div().child(i18n::compact_warning())),
                ),
        );
    }

    if has_review {
        col = col.child(widgets::note(&p, p.review, i18n::review_warning()));
    }

    let open = app.show_confirm_items;
    col = col.child(
        div()
            .id("confirm-items")
            .flex()
            .items_center()
            .gap(px(8.))
            .text_color(p.muted)
            .cursor_pointer()
            .hover(|s| s.text_color(p.text))
            .child(widgets::disclosure(open, p.muted))
            .child(i18n::see_items(count))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.show_confirm_items = !this.show_confirm_items;
                cx.notify();
            })),
    );
    if open {
        col = col.child(
            div()
                .id("confirm-list")
                .max_h(px(260.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(px(4.))
                .children(items.iter().map(|n| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(dot(theme::category_color(n.category)))
                        .child(div().flex_1().min_w_0().truncate().child(n.label.to_string()))
                        .child(div().text_color(p.muted).child(format_bytes(n.size_bytes)))
                })),
        );
    }

    let label = match (bytes > 0, disks.len()) {
        (_, 0) => i18n::delete_size(&format_bytes(bytes)),
        (false, n) => i18n::compact_disks(n),
        (true, _) => i18n::delete_and_compact(&format_bytes(bytes)),
    };
    col = col.child(
        div()
            .mt(px(8.))
            .flex()
            .justify_between()
            .child(widgets::button(app, "back-review", i18n::back_and_review(), Kind::Ghost, true, window, cx, |this, _, _| {
                this.screen = Screen::Results;
            }))
            .child(widgets::button(app, "do-clean", label, Kind::Danger, true, window, cx, |this, _, _| this.start_clean())),
    );

    page(app, ScrollArea::Confirm, col, cx)
}

// ---------------------------------------------------------------- cleaning

pub fn cleaning(app: &mut DevCleaner, window: &mut Window, _cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    if let Some((c, at)) = app.compact.clone() {
        return compacting(&p, &c, at, window).into_any_element();
    }
    let (done, total, label) = app.delete_progress.clone();
    let frac = if total > 0 { done as f32 / total as f32 } else { 0.0 };
    centered(
        column(560.)
            .gap(px(8.))
            .child(div().font_weight(FontWeight::SEMIBOLD).text_color(p.muted).child(i18n::cleaning()))
            .child(big(format!("{:.0}%", frac * 100.0), 72., p.text))
            .child(widgets::progress_bar(&p, frac, 10.))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap(px(16.))
                    .text_color(p.muted)
                    .child(div().flex_none().child(i18n::done_of(done, total)))
                    .child(div().min_w_0().truncate().text_size(px(12.)).child(label)),
            ),
    )
    .into_any_element()
}

fn centered(inner: Div) -> Div {
    div().size_full().flex().items_center().justify_center().px(px(28.)).child(inner)
}

fn compacting(p: &Palette, c: &CompactProgress, at: Instant, window: &Window) -> impl IntoElement {
    let elapsed = c.elapsed_secs + at.elapsed().as_secs();
    let (fast, slow) = c.estimate_secs;
    // Real progress when diskpart's output is readable; otherwise the bar
    // follows the clock against the slow estimate and never fills up.
    let frac = match c.percent {
        Some(pct) => (c.disk as f32 + pct as f32 / 100.0) / c.disks.max(1) as f32,
        None => (elapsed as f32 / slow.max(1) as f32).min(0.95),
    };
    window.request_animation_frame();
    let headline = match c.percent {
        Some(_) => format!("{:.0}%", frac * 100.0),
        None => i18n::duration(elapsed),
    };
    let note = if elapsed > slow { i18n::past_estimate() } else { i18n::compact_slow_note() };
    centered(
        column(600.)
            .gap(px(8.))
            .child(div().font_weight(FontWeight::SEMIBOLD).text_color(p.muted).child(i18n::compacting_title()))
            .child(big(headline, 64., p.text))
            .child(widgets::progress_bar(p, frac, 10.))
            .child(
                div()
                    .flex()
                    .gap(px(8.))
                    .child(i18n::compacting_disk(c.disk + 1, c.disks))
                    .child(div().min_w_0().truncate().text_color(p.muted).child(c.label.to_string())),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.))
                    .text_color(p.muted)
                    .when(c.percent.is_some(), |d| d.child(i18n::elapsed(&i18n::duration(elapsed))))
                    .child(i18n::compact_estimate(fast, slow)),
            )
            .when(c.percent.is_none() && elapsed < 30, |d| {
                d.child(div().text_size(px(12.)).text_color(p.muted).child(i18n::waiting_admin()))
            })
            .child(div().mt(px(10.)).child(widgets::note(p, p.review, note))),
    )
}

// ---------------------------------------------------------------- report

pub fn report(app: &mut DevCleaner, window: &mut Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let freed: u64 = app
        .delete_results
        .iter()
        .filter(|r| r.outcome == DeleteOutcome::Removed)
        .map(|r| r.bytes_freed)
        .sum();
    let count = |o: DeleteOutcome| app.delete_results.iter().filter(|r| r.outcome == o).count();
    let (removed, skipped, failed) = (count(DeleteOutcome::Removed), count(DeleteOutcome::Skipped), count(DeleteOutcome::Failed));

    // The freed amount counts up once when the report opens.
    let t = app
        .report_opened
        .map(|at| (at.elapsed().as_secs_f32() / 1.1).min(1.0))
        .unwrap_or(1.0);
    if t < 1.0 {
        window.request_animation_frame();
    }
    let shown = (freed as f64 * (1.0 - (1.0 - t as f64).powi(3))) as u64;

    let stat = |n: usize, label: &'static str, c: Hsla| {
        widgets::card(&p)
            .px(px(16.))
            .py(px(10.))
            .flex()
            .items_baseline()
            .gap(px(8.))
            .child(big(n.to_string(), 22., c))
            .child(div().text_color(p.muted).child(label))
    };

    let mut results: Vec<_> = app.delete_results.iter().collect();
    results.sort_by_key(|r| {
        (
            match r.outcome {
                DeleteOutcome::Failed => 0,
                DeleteOutcome::Skipped => 1,
                DeleteOutcome::Removed => 2,
            },
            std::cmp::Reverse(r.bytes_freed),
        )
    });

    let list = widgets::card(&p).px(px(16.)).py(px(10.)).child(
        div()
            .id("report-list")
            .max_h(px(320.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(6.))
            .children(results.into_iter().map(|item| {
                let c = match item.outcome {
                    DeleteOutcome::Removed => p.safe,
                    DeleteOutcome::Skipped => p.review,
                    DeleteOutcome::Failed => p.danger,
                };
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child(dot(c))
                            .child(div().flex_none().text_color(c).child(i18n::outcome_label(item.outcome)))
                            .child(div().flex_1().min_w_0().truncate().child(item.label.to_string()))
                            .when(item.bytes_freed > 0, |d| d.child(div().text_color(p.muted).child(format_bytes(item.bytes_freed)))),
                    )
                    .when_some(item.message.as_ref(), |d, m| {
                        d.child(div().pl(px(17.)).text_size(px(12.)).text_color(p.muted).child(m.to_string()))
                    })
            })),
    );

    let col = column(660.)
        .gap(px(14.))
        .child(div().font_weight(FontWeight::SEMIBOLD).text_color(p.muted).child(i18n::report_title()))
        .child(big(format_bytes(shown), 72., p.safe))
        .child(div().text_color(p.muted).child(i18n::freed_on_disk()))
        .child(
            div()
                .flex()
                .gap(px(10.))
                .child(stat(removed, i18n::stat_removed(), p.safe))
                .when(skipped > 0, |d| d.child(stat(skipped, i18n::stat_skipped(), p.review)))
                .when(failed > 0, |d| d.child(stat(failed, i18n::stat_failed(), p.danger))),
        )
        .child(list)
        .child(
            div()
                .mt(px(4.))
                .flex()
                .gap(px(12.))
                .child(widgets::button(app, "report-home", i18n::back_home(), Kind::Ghost, true, window, cx, |this, _, _| {
                    this.tree.clear();
                    this.has_scanned = false;
                    this.go_home();
                }))
                .child(widgets::button(app, "report-new", i18n::new_scan(), Kind::Primary, true, window, cx, |this, _, _| this.start_scan())),
        );
    page(app, ScrollArea::Report, col, cx)
}
