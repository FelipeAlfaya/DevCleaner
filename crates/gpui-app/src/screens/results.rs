//! Results: strata bar, filters, the (virtualized) list and the footer with
//! the amount that will be deleted.

use std::collections::HashMap;

use cleaner_core::{format_bytes, Category, CheckState, Risk, SortKey, VisibleRow};
use gpui::{
    div, prelude::*, px, relative, uniform_list, AnyElement, ClickEvent, Context, ElementId,
    FontWeight, SharedString, Window,
};

use crate::app::{DevCleaner, Menu, Screen};
use crate::i18n;
use crate::theme;
use crate::widgets::{self, Kind};

const PAGE_X: f32 = 28.0;
const ROW_H: f32 = 58.0;
const ROW_GAP: f32 = 6.0;

pub fn render(app: &mut DevCleaner, window: &mut Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let rows = app.tree.visible_rows(&app.filter);
    let list: AnyElement = if rows.is_empty() {
        empty_state(app, window, cx).into_any_element()
    } else {
        let max_root = app.tree.root_nodes().iter().map(|n| n.size_bytes).max().unwrap_or(1).max(1);
        uniform_list(
            "results",
            rows.len(),
            cx.processor(move |this: &mut DevCleaner, range: std::ops::Range<usize>, window, cx| {
                range.map(|i| row(this, rows[i], max_root, window, cx)).collect::<Vec<_>>()
            }),
        )
        .track_scroll(app.list_scroll.clone())
        .flex_1()
        .px(px(PAGE_X - 2.0))
        .into_any_element()
    };

    div()
        .size_full()
        .flex()
        .flex_col()
        .child(
            div()
                .flex_none()
                .px(px(PAGE_X))
                .flex()
                .flex_col()
                .gap(px(10.))
                .child(header(app, window, cx))
                .child(strata(app, cx))
                .child(legend(app))
                .child(toolbar(app, window, cx))
                .child(filter_chips(app, window, cx))
                .pb(px(8.)),
        )
        .child(
            div()
                .relative()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .child(list)
                .child(widgets::scrollbar(app, widgets::ScrollArea::Results, cx)),
        )
        .child(footer(app, window, cx))
}

fn header(app: &mut DevCleaner, window: &mut Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let status: SharedString = if app.scanning {
        i18n::scanning_progress(app.categories_done, app.scan_total).into()
    } else {
        i18n::total_in(&format_bytes(app.tree.total_found()), app.tree.root_nodes().len()).into()
    };
    let action = if app.scanning {
        widgets::chip(app, "stop-scan", i18n::stop_scan(), false, None, window, cx, |this, _, _| this.stop_scan())
    } else {
        widgets::chip(app, "rescan", i18n::scan_again(), false, None, window, cx, |this, _, _| this.start_scan())
    };
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_baseline()
                .gap(px(12.))
                .child(
                    div()
                        .font_family(theme::display_font())
                        .text_size(px(30.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(i18n::results_title()),
                )
                .child(
                    // Spinner + status share a centred row so the spinner sits
                    // on the text's midline, not on the title's baseline.
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .when(app.scanning, |d| d.child(widgets::spinner(p.accent, 14.)))
                        .child(div().text_color(p.muted).child(status)),
                )
                .when(app.scanned_profile != cleaner_core::ScanProfile::Full, |d| {
                    d.child(widgets::pill(&p, i18n::profile_tag(i18n::profile_name(app.scanned_profile)), p.muted))
                }),
        )
        .child(action)
}

/// One segment per category, width = bytes found; the solid part is what is
/// currently selected. Clicking a segment filters that category.
fn strata(app: &mut DevCleaner, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let segs = app.strata_segments();
    let total: u64 = segs.iter().map(|s| s.1).sum();
    let n = segs.len();
    let items: Vec<_> = segs
        .iter()
        .enumerate()
        .map(|(i, &(cat, found, selected))| {
            let color = theme::category_color(cat);
            let frac = found as f32 / total.max(1) as f32;
            let sel = (selected as f32 / found.max(1) as f32).clamp(0.0, 1.0);
            let r_left = if i == 0 { 8.0 } else { 2.0 };
            let r_right = if i + 1 == n { 8.0 } else { 2.0 };
            div()
                .id(ElementId::Name(format!("strata-{cat:?}").into()))
                .relative()
                .h_full()
                .flex_basis(relative(frac))
                .min_w(px(4.))
                .rounded_l(px(r_left))
                .rounded_r(px(r_right))
                .overflow_hidden()
                .bg(theme::mix(color, p.raised, if p.dark { 0.68 } else { 0.62 }))
                .cursor_pointer()
                .hover(|s| s.border_2().border_color(p.text))
                .child(div().absolute().top_0().left_0().h_full().w(relative(sel)).bg(color))
                .tooltip(widgets::tooltip(
                    p,
                    vec![
                        (i18n::category_name(cat).into(), None, true),
                        (i18n::strata_hover(&format_bytes(found), &format_bytes(selected)).into(), None, false),
                        (i18n::click_to_filter().into(), Some(p.muted), false),
                    ],
                ))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    let only = this.filter.categories.len() == 1 && this.filter.categories.contains(&cat);
                    this.filter.categories.clear();
                    if !only {
                        this.filter.categories.insert(cat);
                    }
                    cx.notify();
                }))
        })
        .collect();
    div()
        .w_full()
        .h(px(28.))
        .rounded(px(8.))
        .bg(p.raised)
        .flex()
        .gap(px(2.))
        .children(items)
}

fn legend(app: &DevCleaner) -> impl IntoElement {
    let p = app.palette();
    let segs = app.strata_segments();
    let more = segs.len().saturating_sub(7);
    div()
        .flex()
        .flex_wrap()
        .gap_x(px(14.))
        .gap_y(px(4.))
        .text_size(px(12.))
        .text_color(p.muted)
        .children(segs.into_iter().take(7).map(|(c, found, _)| {
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .child(div().size(px(8.)).rounded_full().bg(theme::category_color(c)))
                .child(format!("{} {}", i18n::category_name(c), format_bytes(found)))
        }))
        .when(more > 0, |d| d.child(i18n::more_categories(more)))
}

fn search_box(app: &DevCleaner, window: &Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let focused = app.search_focus.is_focused(window);
    let text = app.filter.search.clone();
    let content = if text.is_empty() && !focused {
        div().text_color(p.muted).child(i18n::search_placeholder())
    } else {
        div()
            .flex()
            .items_center()
            .min_w_0()
            .child(div().truncate().child(text))
            .when(focused, |d| d.child(div().ml(px(1.)).w(px(1.5)).h(px(16.)).bg(p.accent_ink)))
    };
    div()
        .id("search")
        .track_focus(&app.search_focus)
        .flex()
        .items_center()
        .gap(px(8.))
        .w(px(270.))
        .h(px(32.))
        .px(px(10.))
        .rounded(px(8.))
        .bg(p.raised)
        .border_1()
        .border_color(if focused { p.accent_ink } else { p.line })
        .cursor_text()
        .child(div().text_color(p.muted).text_size(px(13.)).child("⌕"))
        .child(content)
        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
            window.focus(&this.search_focus);
            cx.notify();
        }))
}

fn toolbar(app: &mut DevCleaner, window: &mut Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let only = app.filter.only_selected;
    let sort = app.filter.sort;
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(10.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(search_box(app, window, cx))
                .child(div().text_color(p.muted).child(i18n::sort_by()))
                .child(widgets::dropdown(
                    app,
                    Menu::Sort,
                    "sort-menu",
                    i18n::sort_label(sort),
                    180.0,
                    SortKey::all().iter().map(|s| (*s, SharedString::from(i18n::sort_label(*s)))).collect(),
                    sort,
                    window,
                    cx,
                    |this, s| this.filter.sort = s,
                ))
                .child(widgets::chip(app, "only-selected", i18n::only_selected(), only, None, window, cx, |this, _, _| {
                    this.filter.only_selected = !this.filter.only_selected;
                })),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(widgets::chip(app, "select-visible", i18n::select_visible(), false, None, window, cx, |this, _, _| {
                    let ids: Vec<u64> = this.tree.visible_roots(&this.filter).iter().map(|n| n.id).collect();
                    for id in ids {
                        this.tree.toggle(id, true);
                    }
                }))
                .child(widgets::chip(app, "unselect-visible", i18n::unselect_visible(), false, None, window, cx, |this, _, _| {
                    let ids: Vec<u64> = this.tree.visible_roots(&this.filter).iter().map(|n| n.id).collect();
                    for id in ids {
                        this.tree.toggle(id, false);
                    }
                })),
        )
}

fn filter_chips(app: &mut DevCleaner, window: &mut Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let mut counts: HashMap<Category, usize> = HashMap::new();
    let mut risk_counts: HashMap<Risk, usize> = HashMap::new();
    for n in app.tree.root_nodes() {
        *counts.entry(n.category).or_insert(0) += 1;
        *risk_counts.entry(n.risk).or_insert(0) += 1;
    }
    let found = app.tree.found_by_category();
    let mut cats: Vec<Category> = counts.keys().copied().collect();
    cats.sort_by(|a, b| found.get(b).cmp(&found.get(a)));

    let mut row = div().flex().flex_wrap().items_center().gap(px(6.));
    row = row.child(widgets::chip(app, "cat-all", i18n::all_categories(), app.filter.categories.is_empty(), None, window, cx, |this, _, _| {
        this.filter.categories.clear();
    }));
    for cat in cats {
        let on = app.filter.categories.contains(&cat);
        row = row.child(
            widgets::chip(
                app,
                SharedString::from(format!("cat-{cat:?}")),
                format!("{}  {}", i18n::category_name(cat), counts[&cat]),
                on,
                Some(theme::category_color(cat)),
                window,
                cx,
                move |this, _, _| {
                    if !this.filter.categories.remove(&cat) {
                        this.filter.categories.insert(cat);
                    }
                },
            )
            .tooltip(widgets::simple_tooltip(p, i18n::chip_toggle_hint())),
        );
    }
    row = row.child(div().w(px(1.)).h(px(20.)).mx(px(6.)).bg(p.line));
    for risk in [Risk::Safe, Risk::Review, Risk::Blocked] {
        let n = risk_counts.get(&risk).copied().unwrap_or(0);
        if n == 0 {
            continue;
        }
        let on = app.filter.risks.contains(&risk);
        row = row.child(
            widgets::chip(
                app,
                SharedString::from(format!("risk-{risk:?}")),
                format!("{}  {}", i18n::risk_label(risk), n),
                on,
                Some(p.risk(risk)),
                window,
                cx,
                move |this, _, _| {
                    if !this.filter.risks.remove(&risk) {
                        this.filter.risks.insert(risk);
                    }
                },
            )
            .tooltip(widgets::simple_tooltip(p, i18n::risk_hint(risk))),
        );
    }
    if !app.filter.is_default() {
        row = row.child(
            div()
                .id("clear-filters")
                .ml(px(6.))
                .text_color(p.accent_ink)
                .cursor_pointer()
                .hover(|s| s.underline())
                .child(i18n::clear_filters())
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.clear_filters();
                    cx.notify();
                })),
        );
    }

    let missing: Vec<&str> = Category::all()
        .iter()
        .filter(|c| app.tree.category_status.get(c).is_some_and(|s| !s.available))
        .map(|c| i18n::category_name(*c))
        .collect();

    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(row)
        .when(!missing.is_empty() && !app.scanning, |d| {
            d.child(
                div()
                    .text_size(px(12.))
                    .text_color(p.muted)
                    .child(i18n::nothing_to_clean_in(&missing.join(", "))),
            )
        })
}

fn row(app: &mut DevCleaner, r: VisibleRow, max_root: u64, window: &mut Window, cx: &mut Context<DevCleaner>) -> AnyElement {
    let p = app.palette();
    let Some(node) = app.tree.node(r.id).cloned() else {
        return div().h(px(ROW_H + ROW_GAP)).into_any_element();
    };
    let id = node.id;
    let blocked = node.risk == Risk::Blocked;
    let state = app.tree.check_state(id);
    let open = app.tree.expanded.contains(&id);
    let color = theme::category_color(node.category);
    let denom = match node.parent_id.and_then(|pid| app.tree.node(pid)) {
        Some(parent) => parent.size_bytes.max(1),
        None => max_root,
    };
    let frac = (node.size_bytes as f32 / denom as f32).clamp(0.0, 1.0);
    let fill = if r.depth == 0 { p.surface } else { theme::mix(p.surface, p.bg, 0.5) };

    let disclosure = if node.expandable {
        div()
            .id(ElementId::NamedInteger("exp".into(), id))
            .size(px(24.))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .hover(|s| s.bg(p.raised))
            .cursor_pointer()
            .child(widgets::disclosure(open, p.muted))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.toggle_expand(id);
                cx.stop_propagation();
                cx.notify();
            }))
            .into_any_element()
    } else {
        div().size(px(24.)).flex_none().into_any_element()
    };

    let sub = if r.depth == 0 {
        format!("{}   {}", i18n::category_name(node.category), node.detail)
    } else {
        node.detail.clone()
    };
    let pill_text = if node.is_vhd() { i18n::vhd_pill() } else { i18n::risk_label(node.risk) };

    let mut tip = vec![(SharedString::from(node.label.to_string()), None, true)];
    let reason = node.reason.to_string();
    if !reason.is_empty() {
        tip.push((reason.into(), None, false));
    }
    tip.push((i18n::risk_hint(node.risk).into(), Some(p.risk(node.risk)), false));
    if let Some(m) = node.modified {
        tip.push((i18n::modified_on(&m.format(i18n::date_format()).to_string()).into(), Some(p.muted), false));
    }

    let card = div()
        .id(ElementId::NamedInteger("row".into(), id))
        .relative()
        .ml(px(r.depth as f32 * 26.0))
        .h(px(ROW_H))
        .flex()
        .items_center()
        .gap(px(10.))
        .pl(px(14.))
        .pr(px(16.))
        .rounded(px(10.))
        .bg(fill)
        .border_1()
        .border_color(p.line)
        .hover(|s| s.bg(p.raised))
        .when(!blocked, |d| d.cursor_pointer())
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .bottom_0()
                .w(px(4.))
                .rounded_l(px(10.))
                .bg(color),
        )
        .child(disclosure)
        .child(widgets::checkbox(&p, state, !blocked))
        .child(
            div()
                .ml(px(6.))
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(3.))
                .child(
                    div()
                        .truncate()
                        .text_size(px(14.5))
                        .text_color(if blocked { p.muted } else { p.text })
                        .child(node.label.to_string()),
                )
                .child(div().truncate().text_size(px(12.)).text_color(p.muted).child(sub)),
        )
        .child(widgets::pill(&p, pill_text, p.risk(node.risk)))
        .child(
            div()
                .w(px(116.))
                .flex_none()
                .flex()
                .flex_col()
                .items_end()
                .gap(px(6.))
                .child(
                    div()
                        .font_family(theme::display_font())
                        .text_size(px(17.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(format_bytes(node.size_bytes)),
                )
                .child(
                    div()
                        .w(px(110.))
                        .h(px(4.))
                        .rounded_full()
                        .bg(p.line)
                        .child(div().h_full().min_w(px(2.)).w(relative(frac)).rounded_full().bg(color)),
                ),
        )
        .tooltip(widgets::tooltip(p, tip))
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            if !blocked {
                let next = this.tree.check_state(id) != CheckState::Checked;
                this.tree.toggle(id, next);
                cx.notify();
            }
        }));

    let _ = window;
    div().h(px(ROW_H + ROW_GAP)).pb(px(ROW_GAP)).px(px(2.)).child(card).into_any_element()
}

fn empty_state(app: &mut DevCleaner, window: &mut Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let mut col = div().flex().flex_col().items_center().gap(px(10.)).pt(px(60.));
    if app.scanning {
        col = col
            .child(widgets::spinner(p.accent, 28.))
            .child(div().text_color(p.muted).child(i18n::empty_scanning()));
    } else if app.tree.nodes.is_empty() {
        col = col
            .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).child(i18n::empty_title()))
            .child(div().text_color(p.muted).child(i18n::empty_body()));
    } else {
        col = col
            .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).child(i18n::empty_filtered()))
            .child(widgets::button(app, "empty-clear", i18n::clear_filters(), Kind::Ghost, true, window, cx, |this, _, _| this.clear_filters()));
    }
    col
}

fn footer(app: &mut DevCleaner, window: &mut Window, cx: &mut Context<DevCleaner>) -> impl IntoElement {
    let p = app.palette();
    let (count, bytes) = app.tree.selected_summary();
    let disks = app.tree.selected_vhds().len();
    let total = app.tree.total_found();
    let pct = if total > 0 { bytes as f64 / total as f64 * 100.0 } else { 0.0 };
    let enabled = count > 0 && !app.scanning;
    let btn = widgets::button(app, "review-clean", i18n::clean_cta(), Kind::Primary, enabled, window, cx, |this, _, _| {
        this.show_confirm_items = false;
        this.screen = Screen::Confirm;
    });
    let btn = if enabled {
        btn
    } else if app.scanning {
        btn.tooltip(widgets::simple_tooltip(p, i18n::wait_for_scan()))
    } else {
        btn.tooltip(widgets::simple_tooltip(p, i18n::select_something()))
    };

    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px(px(PAGE_X))
        .py(px(14.))
        .bg(p.surface)
        .border_t_1()
        .border_color(p.line)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(18.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(div().text_size(px(12.)).text_color(p.muted).child(i18n::will_delete()))
                        .child(
                            div()
                                .font_family(theme::display_font())
                                .text_size(px(32.))
                                .line_height(px(36.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.accent_ink)
                                .child(format_bytes(bytes)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(i18n::marked(count))
                        .child(div().text_size(px(12.)).text_color(p.muted).child(i18n::share_of_found(pct, &format_bytes(total))))
                        .when(disks > 0, |d| d.child(div().text_size(px(12.)).text_color(p.review).child(i18n::plus_disks(disks)))),
                ),
        )
        .child(btn)
}
