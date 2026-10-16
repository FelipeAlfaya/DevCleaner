use std::collections::HashMap;

use cleaner_core::scanners;
use cleaner_core::{
    format_bytes, start_delete, CompactProgress, Lang, ScanProfile, start_scan, Category, CheckState, DeleteEvent, DeleteHandle,
    DeleteOutcome, DeleteResultItem, Risk, ScanEvent, ScanHandle, ScanNode, ScanTree, Settings,
    SortKey, ViewFilter, VisibleRow,
};
use eframe::egui::{
    self, Align, Color32, CornerRadius, Layout, Pos2, Rect, RichText, Sense, Stroke, StrokeKind,
    Vec2,
};

use crate::i18n;
use crate::theme::{self, body, display, mix, strong, Palette, StrataSegment};

const ROW_HEIGHT: f32 = 58.0;
const ROW_GAP: f32 = 6.0;
const PAGE_MARGIN: i8 = 28;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Home,
    Results,
    Confirm,
    Cleaning,
    Report,
}

enum RowAction {
    Toggle(u64, bool),
    Expand(u64),
    Collapse(u64),
}

pub struct CleanerApp {
    settings: Settings,
    screen: Screen,
    tree: ScanTree,
    scan_handle: Option<ScanHandle>,
    delete_handle: Option<DeleteHandle>,
    delete_results: Vec<DeleteResultItem>,
    delete_progress: (usize, usize, String),
    /// Latest compaction progress and when it arrived (to tick the clock
    /// between updates).
    compact: Option<(CompactProgress, std::time::Instant)>,
    filter: ViewFilter,
    scanning: bool,
    cleaning: bool,
    categories_done: usize,
    /// Categories and profile of the last scan started.
    scan_total: usize,
    scanned_profile: ScanProfile,
    has_scanned: bool,
    expand_tx: crossbeam_channel::Sender<Vec<ScanNode>>,
    expand_rx: crossbeam_channel::Receiver<Vec<ScanNode>>,
}

impl CleanerApp {
    pub fn new(cc: &eframe::CreationContext<'_>, settings: Settings) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        cleaner_core::i18n::set_lang(settings.effective_language());
        let (expand_tx, expand_rx) = crossbeam_channel::unbounded();
        Self {
            settings,
            screen: Screen::Home,
            tree: ScanTree::default(),
            scan_handle: None,
            delete_handle: None,
            delete_results: Vec::new(),
            delete_progress: (0, 0, String::new()),
            compact: None,
            filter: ViewFilter::default(),
            scanning: false,
            cleaning: false,
            categories_done: 0,
            scan_total: 0,
            scanned_profile: ScanProfile::Full,
            has_scanned: false,
            expand_tx,
            expand_rx,
        }
    }

    fn palette(&self) -> Palette {
        Palette::new(self.settings.dark_theme)
    }

    // ------------------------------------------------------------ workers

    fn start_scan(&mut self) {
        if let Some(h) = self.scan_handle.take() {
            h.cancel();
        }
        self.tree.clear();
        self.categories_done = 0;
        self.scan_total = self.settings.scan_categories().len();
        self.scanned_profile = self.settings.profile;
        self.scanning = true;
        self.has_scanned = true;
        self.screen = Screen::Results;
        self.scan_handle = Some(start_scan(self.settings.clone()));
    }

    fn stop_scan(&mut self) {
        if let Some(h) = self.scan_handle.take() {
            h.cancel();
        }
        self.scanning = false;
    }

    fn poll_scan(&mut self) {
        let mut finished = false;
        let mut error: Option<String> = None;
        if let Some(handle) = &self.scan_handle {
            while let Ok(event) = handle.events.try_recv() {
                match event {
                    ScanEvent::CategoryStatus(s) => self.tree.set_category_status(s),
                    ScanEvent::NodesBatch(batch) => self.tree.ingest_nodes(batch),
                    ScanEvent::CategoryFinished { .. } => self.categories_done += 1,
                    ScanEvent::ScanFinished => finished = true,
                    ScanEvent::ScanError { message } => {
                        finished = true;
                        error = Some(message);
                    }
                }
            }
        }
        if finished {
            self.scanning = false;
            self.scan_handle = None;
            if let Some(message) = error {
                eprintln!("scan error: {message}");
            }
        }
    }

    fn poll_expand(&mut self) {
        while let Ok(batch) = self.expand_rx.try_recv() {
            self.tree.ingest_nodes(batch);
        }
    }

    fn request_expand(&mut self, node: ScanNode) {
        self.tree.expanded.insert(node.id);
        if self.tree.children_loaded.contains(&node.id) {
            return;
        }
        self.tree.children_loaded.insert(node.id);
        let settings = self.settings.clone();
        let expand_tx = self.expand_tx.clone();
        std::thread::spawn(move || {
            let (event_tx, event_rx) = crossbeam_channel::unbounded();
            scanners::expand_node(&node, &settings, &event_tx);
            while let Ok(ScanEvent::NodesBatch(batch)) = event_rx.try_recv() {
                let _ = expand_tx.send(batch);
            }
        });
    }

    fn start_clean(&mut self) {
        let targets = self.tree.selected_ids();
        let nodes = self.tree.nodes.clone();
        self.delete_results.clear();
        self.delete_progress = (0, targets.len(), String::new());
        self.compact = None;
        self.cleaning = true;
        self.screen = Screen::Cleaning;
        self.delete_handle = Some(start_delete(nodes, targets));
    }

    fn poll_delete(&mut self) {
        let mut finished = false;
        if let Some(handle) = &self.delete_handle {
            while let Ok(ev) = handle.events.try_recv() {
                match ev {
                    DeleteEvent::Progress {
                        done,
                        total,
                        current_label,
                    } => self.delete_progress = (done, total, current_label.to_string()),
                    DeleteEvent::Item(item) => self.delete_results.push(item),
                    DeleteEvent::Compacting(c) => self.compact = Some((c, std::time::Instant::now())),
                    DeleteEvent::Finished => finished = true,
                }
            }
        }
        if finished {
            self.cleaning = false;
            self.delete_handle = None;
            // What was found is out of date now; the report keeps its own list.
            self.tree.clear();
            self.has_scanned = false;
            self.screen = Screen::Report;
        }
    }

    fn save_settings(&self) {
        let _ = self.settings.save();
    }

    fn set_language(&mut self, lang: Lang) {
        self.settings.language = Some(lang);
        cleaner_core::i18n::set_lang(lang);
        self.save_settings();
    }

    fn ui_profile_picker(&mut self, ui: &mut egui::Ui, p: &Palette) {
        ui.label(RichText::new(i18n::scan_profile()).font(strong(13.0)).color(p.muted));
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            for &prof in ScanProfile::all() {
                let on = self.settings.profile == prof;
                if theme::chip(ui, p, i18n::profile_name(prof), on, None)
                    .on_hover_text(i18n::profile_hint(prof))
                    .clicked()
                    && !on
                {
                    self.settings.profile = prof;
                    self.save_settings();
                }
            }
        });
        ui.label(RichText::new(i18n::profile_hint(self.settings.profile)).color(p.muted).small());
    }

    fn language_menu(&mut self, ui: &mut egui::Ui) {
        let current = cleaner_core::i18n::lang();
        let mut chosen = current;
        egui::ComboBox::from_id_salt("language")
            .selected_text(current.code().to_uppercase())
            .width(64.0)
            .show_ui(ui, |ui| {
                for &l in Lang::all() {
                    ui.selectable_value(&mut chosen, l, l.native_name());
                }
            })
            .response
            .on_hover_text(i18n::language());
        if chosen != current {
            self.set_language(chosen);
        }
    }

    /// Categories ordered by bytes found, heaviest first.
    fn strata_segments(&self) -> Vec<StrataSegment> {
        let found = self.tree.found_by_category();
        let selected = self.tree.selected_by_category();
        let mut segs: Vec<StrataSegment> = found
            .into_iter()
            .filter(|(_, b)| *b > 0)
            .map(|(category, found)| StrataSegment {
                category,
                found,
                selected: selected.get(&category).copied().unwrap_or(0),
            })
            .collect();
        segs.sort_by(|a, b| b.found.cmp(&a.found));
        segs
    }

    // ------------------------------------------------------------ chrome

    fn ui_top_bar(&mut self, ctx: &egui::Context, p: &Palette) {
        egui::TopBottomPanel::top("top")
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(egui::Margin::symmetric(PAGE_MARGIN, 14)),
            )
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(24.0, 22.0), Sense::hover());
                    theme::paint_logo(ui.painter(), rect, p);
                    ui.add_space(4.0);
                    ui.label(RichText::new(i18n::app_title()).font(display(20.0)).color(p.text));
                    ui.add_space(24.0);

                    let busy = self.cleaning;
                    let in_results = matches!(self.screen, Screen::Results | Screen::Confirm);
                    if !busy {
                        if theme::chip(ui, p, i18n::nav_home(), self.screen == Screen::Home, None).clicked() {
                            self.screen = Screen::Home;
                        }
                        let has_results = !self.tree.nodes.is_empty() || self.scanning;
                        if has_results
                            && theme::chip(ui, p, i18n::nav_results(), in_results, None).clicked()
                        {
                            self.screen = Screen::Results;
                        }
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let label = i18n::theme_toggle(self.settings.dark_theme);
                        if theme::chip(ui, p, label, false, None).clicked() {
                            self.settings.dark_theme = !self.settings.dark_theme;
                            self.save_settings();
                        }
                        self.language_menu(ui);
                        if self.scanning && self.screen != Screen::Results {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(i18n::scanning_short(
                                    self.categories_done,
                                    self.scan_total,
                                ))
                                .color(p.muted),
                            );
                            ui.spinner();
                        }
                    });
                });
            });
    }

    // ------------------------------------------------------------ home

    fn ui_home(&mut self, ui: &mut egui::Ui, p: &Palette) {
        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            ui.add_space(8.0);
            let avail = ui.available_width();
            let wide = avail > 760.0;
            let text_w = if wide { (avail * 0.46).min(480.0) } else { avail };

            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(text_w);
                    ui.add_space(18.0);
                    ui.label(
                        RichText::new(i18n::hero_title())
                            .font(display(34.0))
                            .color(p.text),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(i18n::hero_body())
                        .font(body(15.0))
                        .color(p.muted),
                    );
                    ui.add_space(18.0);
                    self.ui_profile_picker(ui, p);
                    ui.add_space(16.0);

                    let total = self.tree.total_found();
                    let can_scan = !self.settings.scan_categories().is_empty();
                    ui.horizontal(|ui| {
                        if self.scanning {
                            if theme::primary_button(ui, p, &format!("  {}  ", i18n::view_scan_in_progress()), true).clicked() {
                                self.screen = Screen::Results;
                            }
                        } else if self.has_scanned && !self.tree.nodes.is_empty() {
                            if theme::primary_button(ui, p, &format!("  {}  ", i18n::view_results()), true).clicked() {
                                self.screen = Screen::Results;
                            }
                            if theme::ghost_button(ui, p, &format!("  {}  ", i18n::scan_again())).clicked() && can_scan {
                                self.start_scan();
                            }
                        } else if theme::primary_button(ui, p, &format!("  {}  ", i18n::scan_cta()), can_scan)
                            .on_disabled_hover_text(i18n::empty_profile())
                            .clicked()
                        {
                            self.start_scan();
                        }
                    });
                    if self.has_scanned && total > 0 {
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format_bytes(total)).font(display(22.0)).color(p.accent_ink()));
                            ui.label(
                                RichText::new(i18n::found_summary(self.tree.root_nodes().len()))
                                .color(p.muted),
                            );
                        });
                    }
                });

                if wide {
                    ui.add_space(24.0);
                    let w = ui.available_width() - 14.0;
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 250.0), Sense::hover());
                    theme::paint_landscape(ui, rect, &self.landscape_layers(p), p);
                }
            });

            ui.add_space(30.0);
            ui.label(RichText::new(i18n::where_we_look()).font(strong(16.0)).color(p.text));
            ui.add_space(4.0);
            self.ui_category_tiles(ui, p);

            ui.add_space(26.0);
            ui.label(RichText::new(i18n::preferences()).font(strong(16.0)).color(p.text));
            ui.add_space(4.0);
            theme::card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(i18n::temp_age_title()).font(strong(14.0)));
                        ui.label(
                            RichText::new(i18n::temp_age_body())
                                .color(p.muted)
                                .small(),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let resp = ui.add(
                            egui::Slider::new(&mut self.settings.temp_min_age_days, 0..=90)
                                .suffix(i18n::days_suffix()),
                        );
                        if resp.drag_stopped() || (resp.changed() && !resp.dragged()) {
                            self.save_settings();
                        }
                    });
                });
                ui.add_space(6.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::ZERO, p.line);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(i18n::language()).font(strong(14.0)));
                        ui.label(RichText::new(i18n::language_body()).color(p.muted).small());
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let current = cleaner_core::i18n::lang();
                        for &l in Lang::all().iter().rev() {
                            if theme::chip(ui, p, l.native_name(), current == l, None).clicked() && current != l {
                                self.set_language(l);
                            }
                        }
                    });
                });
            });
            ui.add_space(24.0);
        });
    }

    /// Layers for the hero illustration: real proportions after a scan,
    /// a quiet placeholder before.
    fn landscape_layers(&self, p: &Palette) -> Vec<(Color32, f32, Option<(String, String)>)> {
        let segs = self.strata_segments();
        if segs.is_empty() {
            let alpha = if p.dark { 0.5 } else { 0.4 };
            return [
                (Category::Temp, 0.7),
                (Category::Yarn, 0.9),
                (Category::Gradle, 1.2),
                (Category::Cargo, 1.0),
                (Category::Docker, 1.6),
                (Category::Maven, 1.1),
            ]
            .into_iter()
            .map(|(c, w)| (mix(theme::category_color(c), p.bg, alpha), w, None))
            .collect();
        }
        // Heaviest at the bottom, like the oldest sediment. Tiny layers keep a
        // minimum thickness so they stay visible.
        let total: u64 = segs.iter().map(|s| s.found).sum();
        let mut layers: Vec<_> = segs
            .iter()
            .take(8)
            .map(|s| {
                let w = (s.found as f32 / total.max(1) as f32).max(0.06);
                (
                    theme::category_color(s.category),
                    w,
                    Some((i18n::category_name(s.category).to_string(), format_bytes(s.found))),
                )
            })
            .collect();
        layers.reverse();
        layers
    }

    fn ui_category_tiles(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let avail = ui.available_width();
        let min_w = 180.0;
        let gap = 10.0;
        let cols = ((avail + gap) / (min_w + gap)).floor().max(1.0) as usize;
        let tile_w = (avail - gap * (cols as f32 - 1.0)) / cols as f32;
        let tile_h = 74.0;

        let mut cats: Vec<Category> = Category::all().to_vec();
        if self.has_scanned {
            let found = self.tree.found_by_category();
            cats.sort_by(|a, b| {
                let fa = found.get(a).copied().unwrap_or(0);
                let fb = found.get(b).copied().unwrap_or(0);
                fb.cmp(&fa)
            });
        }

        let in_profile_list = self.settings.scan_categories();
        let custom = self.settings.profile == ScanProfile::Custom;
        let mut go_to: Option<Category> = None;
        let mut toggle: Option<Category> = None;
        for row in cats.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &cat in row {
                    let (rect, resp) = ui.allocate_exact_size(Vec2::new(tile_w, tile_h), Sense::click());
                    let status = self.tree.category_status.get(&cat);
                    let available = status.is_some_and(|s| s.available);
                    let in_profile = in_profile_list.contains(&cat);
                    let clickable = custom || (available && in_profile);
                    let painter = ui.painter();
                    let fill = match (in_profile, resp.hovered() && clickable) {
                        (_, true) => p.raised,
                        (true, false) => p.surface,
                        (false, false) => p.bg,
                    };
                    let border = if custom && in_profile { p.accent_ink() } else { p.line };
                    painter.rect(rect, CornerRadius::same(12), fill, Stroke::new(1.0_f32, border), StrokeKind::Inside);
                    let color = theme::category_color(cat);
                    let stripe = Rect::from_min_size(rect.min + Vec2::new(14.0, 16.0), Vec2::new(4.0, tile_h - 32.0));
                    let stripe_color = if !in_profile || (status.is_some() && !available) {
                        mix(color, p.surface, 0.7)
                    } else {
                        color
                    };
                    painter.rect_filled(stripe, CornerRadius::same(2), stripe_color);

                    let x = rect.left() + 30.0;
                    painter.text(
                        Pos2::new(x, rect.top() + 16.0),
                        egui::Align2::LEFT_TOP,
                        i18n::category_name(cat),
                        strong(14.0),
                        if in_profile { p.text } else { p.muted },
                    );
                    let (value, value_font, value_color) = match status {
                        _ if !in_profile => (i18n::outside_profile().to_string(), body(13.0), p.muted),
                        Some(s) if s.available && s.total_bytes == 0 && s.disk_bytes > 0 => {
                            (i18n::disk_size(&format_bytes(s.disk_bytes)), body(13.0), p.text)
                        }
                        None if self.scanning => (i18n::tile_searching().to_string(), body(13.0), p.muted),
                        None => (i18n::tile_not_checked().to_string(), body(13.0), p.muted),
                        Some(s) if s.available => (format_bytes(s.total_bytes), display(20.0), p.text),
                        Some(_) => (i18n::tile_nothing().to_string(), body(13.0), p.muted),
                    };
                    painter.text(
                        Pos2::new(x, rect.bottom() - 14.0),
                        egui::Align2::LEFT_BOTTOM,
                        value,
                        value_font,
                        value_color,
                    );
                    if let Some(s) = status.filter(|s| s.available) {
                        painter.text(
                            Pos2::new(rect.right() - 14.0, rect.bottom() - 16.0),
                            egui::Align2::RIGHT_BOTTOM,
                            i18n::items(s.item_count),
                            body(12.0),
                            p.muted,
                        );
                    }
                    let resp = if custom {
                        let hint = if in_profile { i18n::tile_toggle_exclude() } else { i18n::tile_toggle_include() };
                        resp.on_hover_text(hint).on_hover_cursor(egui::CursorIcon::PointingHand)
                    } else if let Some(reason) = status.and_then(|s| s.unavailable_reason.as_ref()) {
                        resp.on_hover_text(reason.to_string())
                    } else if available && in_profile {
                        resp.on_hover_text(i18n::tile_hover()).on_hover_cursor(egui::CursorIcon::PointingHand)
                    } else {
                        resp
                    };
                    if resp.clicked() {
                        if custom {
                            toggle = Some(cat);
                        } else if available && in_profile {
                            go_to = Some(cat);
                        }
                    }
                }
            });
            ui.add_space(gap - ui.spacing().item_spacing.y);
        }
        if let Some(cat) = toggle {
            let list = &mut self.settings.custom_categories;
            if let Some(i) = list.iter().position(|c| *c == cat) {
                list.remove(i);
            } else {
                list.push(cat);
            }
            self.save_settings();
        }
        if let Some(cat) = go_to {
            self.filter.categories.clear();
            self.filter.categories.insert(cat);
            self.screen = Screen::Results;
        }
    }

    // ------------------------------------------------------------ results

    fn ui_results_footer(&mut self, ctx: &egui::Context, p: &Palette) {
        let (count, bytes) = self.tree.selected_summary();
        let disks = self.tree.selected_vhds().len();
        let total = self.tree.total_found();
        egui::TopBottomPanel::bottom("footer")
            .frame(
                egui::Frame::new()
                    .fill(p.surface)
                    .stroke(Stroke::new(1.0_f32, p.line))
                    .inner_margin(egui::Margin::symmetric(PAGE_MARGIN, 14)),
            )
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new(i18n::will_delete()).color(p.muted).small());
                        ui.label(RichText::new(format_bytes(bytes)).font(display(32.0)).color(p.accent_ink()));
                    });
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.add_space(8.0);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(RichText::new(i18n::marked(count)).color(p.text));
                        let pct = if total > 0 { bytes as f64 / total as f64 * 100.0 } else { 0.0 };
                        ui.label(
                            RichText::new(i18n::share_of_found(pct, &format_bytes(total)))
                                .color(p.muted)
                                .small(),
                        );
                        if disks > 0 {
                            ui.label(RichText::new(i18n::plus_disks(disks)).color(p.review).small());
                        }
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let enabled = count > 0 && !self.scanning;
                        let resp = theme::primary_button(ui, p, &format!("  {}  ", i18n::clean_cta()), enabled);
                        let resp = if self.scanning {
                            resp.on_disabled_hover_text(i18n::wait_for_scan())
                        } else {
                            resp.on_disabled_hover_text(i18n::select_something())
                        };
                        if resp.clicked() {
                            self.screen = Screen::Confirm;
                        }
                    });
                });
            });
    }

    fn ui_results(&mut self, ui: &mut egui::Ui, p: &Palette) {
        // Header
        ui.horizontal(|ui| {
            ui.label(RichText::new(i18n::results_title()).font(display(28.0)).color(p.text));
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.add_space(9.0);
                ui.horizontal(|ui| {
                    if self.scanning {
                        ui.spinner();
                        ui.label(
                            RichText::new(i18n::scanning_progress(
                                self.categories_done,
                                self.scan_total,
                            ))
                            .color(p.muted),
                        );
                    } else {
                        ui.label(
                            RichText::new(i18n::total_in(
                                &format_bytes(self.tree.total_found()),
                                self.tree.root_nodes().len(),
                            ))
                            .color(p.muted),
                        );
                    }
                    if self.scanned_profile != ScanProfile::Full {
                        ui.label(
                            RichText::new(i18n::profile_tag(i18n::profile_name(self.scanned_profile)))
                                .color(p.muted),
                        );
                    }
                });
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.scanning {
                    if theme::chip(ui, p, i18n::stop_scan(), false, None).clicked() {
                        self.stop_scan();
                    }
                } else if theme::chip(ui, p, i18n::scan_again(), false, None).clicked() {
                    self.start_scan();
                }
            });
        });
        ui.add_space(6.0);

        // Strata bar
        let segs = self.strata_segments();
        if let Some(cat) = theme::strata_bar(ui, p, &segs, 26.0) {
            if self.filter.categories.len() == 1 && self.filter.categories.contains(&cat) {
                self.filter.categories.clear();
            } else {
                self.filter.categories.clear();
                self.filter.categories.insert(cat);
            }
        }
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            for s in segs.iter().take(7) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                    ui.painter().circle_filled(r.center(), 4.0, theme::category_color(s.category));
                    ui.label(
                        RichText::new(format!("{} {}", i18n::category_name(s.category), format_bytes(s.found)))
                            .color(p.muted)
                            .small(),
                    );
                });
            }
            if segs.len() > 7 {
                ui.label(RichText::new(i18n::more_categories(segs.len() - 7)).color(p.muted).small());
            }
        });
        ui.add_space(10.0);

        self.ui_toolbar(ui, p);
        ui.add_space(8.0);

        // List
        let rows = self.tree.visible_rows(&self.filter);
        if rows.is_empty() {
            self.ui_empty_results(ui, p);
            return;
        }
        let max_root = self
            .tree
            .root_nodes()
            .iter()
            .map(|n| n.size_bytes)
            .max()
            .unwrap_or(1)
            .max(1);

        let mut actions = Vec::new();
        ui.spacing_mut().item_spacing.y = ROW_GAP;
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show_rows(ui, ROW_HEIGHT, rows.len(), |ui, range| {
                for row in &rows[range] {
                    self.draw_row(ui, p, *row, max_root, &mut actions);
                }
            });

        for action in actions {
            match action {
                RowAction::Toggle(id, v) => self.tree.toggle(id, v),
                RowAction::Collapse(id) => {
                    self.tree.expanded.remove(&id);
                }
                RowAction::Expand(id) => {
                    if let Some(node) = self.tree.node(id).cloned() {
                        self.request_expand(node);
                    }
                }
            }
        }
    }

    fn ui_toolbar(&mut self, ui: &mut egui::Ui, p: &Palette) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.filter.search)
                    .hint_text(i18n::search_placeholder())
                    .desired_width(260.0)
                    .margin(egui::Margin::symmetric(10, 6)),
            );
            ui.add_space(4.0);
            ui.label(RichText::new(i18n::sort_by()).color(p.muted));
            egui::ComboBox::from_id_salt("sort")
                .selected_text(i18n::sort_label(self.filter.sort))
                .width(175.0)
                .height(320.0)
                .show_ui(ui, |ui| {
                    for &s in SortKey::all() {
                        ui.selectable_value(&mut self.filter.sort, s, i18n::sort_label(s));
                    }
                });
            ui.add_space(4.0);
            let only = self.filter.only_selected;
            if theme::chip(ui, p, i18n::only_selected(), only, None).clicked() {
                self.filter.only_selected = !only;
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::chip(ui, p, i18n::unselect_visible(), false, None).clicked() {
                    for n in self.tree.visible_roots(&self.filter).iter().map(|n| n.id).collect::<Vec<_>>() {
                        self.tree.toggle(n, false);
                    }
                }
                if theme::chip(ui, p, i18n::select_visible(), false, None).clicked() {
                    for n in self.tree.visible_roots(&self.filter).iter().map(|n| n.id).collect::<Vec<_>>() {
                        self.tree.toggle(n, true);
                    }
                }
            });
        });
        ui.add_space(4.0);

        // Category + risk chips
        let mut counts: HashMap<Category, usize> = HashMap::new();
        let mut risk_counts: HashMap<Risk, usize> = HashMap::new();
        for n in self.tree.root_nodes() {
            *counts.entry(n.category).or_insert(0) += 1;
            *risk_counts.entry(n.risk).or_insert(0) += 1;
        }
        let found = self.tree.found_by_category();
        let mut cats: Vec<Category> = counts.keys().copied().collect();
        cats.sort_by(|a, b| found.get(b).cmp(&found.get(a)));

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
            if theme::chip(ui, p, i18n::all_categories(), self.filter.categories.is_empty(), None).clicked() {
                self.filter.categories.clear();
            }
            for cat in cats {
                let on = self.filter.categories.contains(&cat);
                let label = format!("{}  {}", i18n::category_name(cat), counts[&cat]);
                if theme::chip(ui, p, &label, on, Some(theme::category_color(cat)))
                    .on_hover_text(i18n::chip_toggle_hint())
                    .clicked()
                {
                    if on {
                        self.filter.categories.remove(&cat);
                    } else {
                        self.filter.categories.insert(cat);
                    }
                }
            }

            ui.add_space(10.0);
            let (r, _) = ui.allocate_exact_size(Vec2::new(1.0, 20.0), Sense::hover());
            ui.painter().rect_filled(r, CornerRadius::ZERO, p.line);
            ui.add_space(10.0);

            for risk in [Risk::Safe, Risk::Review, Risk::Blocked] {
                let n = risk_counts.get(&risk).copied().unwrap_or(0);
                if n == 0 {
                    continue;
                }
                let on = self.filter.risks.contains(&risk);
                let label = format!("{}  {}", i18n::risk_label(risk), n);
                if theme::chip(ui, p, &label, on, Some(p.risk(risk)))
                    .on_hover_text(i18n::risk_hint(risk))
                    .clicked()
                {
                    if on {
                        self.filter.risks.remove(&risk);
                    } else {
                        self.filter.risks.insert(risk);
                    }
                }
            }

            if !self.filter.is_default() {
                ui.add_space(6.0);
                if ui
                    .add(egui::Label::new(RichText::new(i18n::clear_filters()).color(p.accent_ink())).sense(Sense::click()))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    let sort = self.filter.sort;
                    self.filter = ViewFilter { sort, ..Default::default() };
                }
            }
        });

        // Categories that had nothing to clean
        let missing: Vec<&str> = Category::all()
            .iter()
            .filter(|c| self.tree.category_status.get(c).is_some_and(|s| !s.available))
            .map(|c| i18n::category_name(*c))
            .collect();
        if !missing.is_empty() && !self.scanning {
            ui.add_space(2.0);
            ui.label(
                RichText::new(i18n::nothing_to_clean_in(&missing.join(", ")))
                    .color(p.muted)
                    .small(),
            );
        }
    }

    fn ui_empty_results(&mut self, ui: &mut egui::Ui, p: &Palette) {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            if self.scanning {
                ui.spinner();
                ui.add_space(8.0);
                ui.label(RichText::new(i18n::empty_scanning()).color(p.muted));
            } else if self.tree.nodes.is_empty() {
                ui.label(RichText::new(i18n::empty_title()).font(strong(18.0)));
                ui.label(RichText::new(i18n::empty_body()).color(p.muted));
            } else {
                ui.label(RichText::new(i18n::empty_filtered()).font(strong(18.0)));
                ui.add_space(6.0);
                if theme::ghost_button(ui, p, &format!("  {}  ", i18n::clear_filters())).clicked() {
                    let sort = self.filter.sort;
                    self.filter = ViewFilter { sort, ..Default::default() };
                }
            }
        });
    }

    fn draw_row(&self, ui: &mut egui::Ui, p: &Palette, row: VisibleRow, max_root: u64, actions: &mut Vec<RowAction>) {
        let Some(node) = self.tree.node(row.id) else { return };
        let width = ui.available_width() - 6.0;
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, ROW_HEIGHT), Sense::click());
        if !ui.is_rect_visible(rect) {
            return;
        }
        let blocked = node.risk == Risk::Blocked;
        let state = self.tree.check_state(node.id);
        let indent = row.depth as f32 * 26.0;
        let card = Rect::from_min_max(Pos2::new(rect.left() + indent, rect.top()), rect.max);
        let cat_color = theme::category_color(node.category);

        let exp_rect = Rect::from_center_size(Pos2::new(card.left() + 22.0, card.center().y), Vec2::splat(24.0));
        let exp_resp = node
            .expandable
            .then(|| ui.interact(exp_rect, ui.id().with(("exp", node.id)), Sense::click()));

        let painter = ui.painter();
        let hovered = resp.hovered() || exp_resp.as_ref().is_some_and(|r| r.hovered());
        let fill = match (row.depth, hovered) {
            (0, false) => p.surface,
            (0, true) => p.raised,
            (_, false) => mix(p.surface, p.bg, 0.5),
            (_, true) => p.raised,
        };
        let stroke = Stroke::new(1.0_f32, p.line);
        painter.rect(card, CornerRadius::same(10), fill, stroke, StrokeKind::Inside);
        // Category stripe
        painter.rect_filled(
            Rect::from_min_size(card.min + Vec2::new(0.0, 0.0), Vec2::new(4.0, card.height())),
            CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 },
            cat_color,
        );

        // Disclosure
        if let Some(er) = &exp_resp {
            let open = ui.ctx().animate_bool(ui.id().with(("open", node.id)), self.tree.expanded.contains(&node.id));
            let col = if er.hovered() { p.text } else { p.muted };
            theme::paint_disclosure(painter, exp_rect.center(), open, col);
            if er.clicked() {
                if self.tree.expanded.contains(&node.id) {
                    actions.push(RowAction::Collapse(node.id));
                } else {
                    actions.push(RowAction::Expand(node.id));
                }
            }
        }

        // Checkbox
        let cb = Rect::from_center_size(Pos2::new(card.left() + 50.0, card.center().y), Vec2::splat(18.0));
        theme::paint_checkbox(painter, cb, state, !blocked, p);

        // Right side: size + bar, then risk pill
        let right = card.right() - 16.0;
        let size_text = format_bytes(node.size_bytes);
        painter.text(
            Pos2::new(right, card.top() + 11.0),
            egui::Align2::RIGHT_TOP,
            &size_text,
            display(17.0),
            p.text,
        );
        let bar_w = 110.0;
        let denom = match node.parent_id.and_then(|pid| self.tree.node(pid)) {
            Some(parent) => parent.size_bytes.max(1),
            None => max_root,
        };
        let frac = (node.size_bytes as f32 / denom as f32).clamp(0.0, 1.0);
        let track = Rect::from_min_size(Pos2::new(right - bar_w, card.bottom() - 17.0), Vec2::new(bar_w, 4.0));
        painter.rect_filled(track, CornerRadius::same(2), p.line);
        let mut fillr = track;
        fillr.set_width((bar_w * frac).max(2.0));
        painter.rect_filled(fillr, CornerRadius::same(2), cat_color);

        let pill_rect = theme::paint_pill(
            painter,
            Pos2::new(right - bar_w - 18.0, card.center().y),
            if node.is_vhd() { i18n::vhd_pill() } else { i18n::risk_label(node.risk) },
            p.risk(node.risk),
            p,
        );

        // Label + path
        let text_left = card.left() + 72.0;
        let text_w = pill_rect.left() - text_left - 16.0;
        let label_color = if blocked { p.muted } else { p.text };
        let label = theme::truncated(ui, &node.label.to_string(), body(14.5), label_color, text_w);
        painter.galley(Pos2::new(text_left, card.top() + 10.0), label, label_color);
        let mut sub = String::new();
        if row.depth == 0 {
            sub.push_str(i18n::category_name(node.category));
            sub.push_str("   ");
        }
        sub.push_str(&node.detail);
        let path = theme::truncated(ui, &sub, body(12.0), p.muted, text_w);
        painter.galley(Pos2::new(text_left, card.bottom() - 24.0), path, p.muted);

        let resp = resp.on_hover_ui_at_pointer(|ui| {
            ui.set_max_width(360.0);
            ui.label(RichText::new(node.label.to_string()).font(strong(14.0)));
            let reason = node.reason.to_string();
            if !reason.is_empty() {
                ui.label(reason);
            }
            ui.label(RichText::new(i18n::risk_hint(node.risk)).color(p.risk(node.risk)));
            if let Some(m) = node.modified {
                ui.label(
                    RichText::new(i18n::modified_on(&m.format(i18n::date_format()).to_string()))
                        .color(p.muted)
                        .small(),
                );
            }
        });
        if resp.clicked() && !blocked {
            actions.push(RowAction::Toggle(node.id, state != CheckState::Checked));
        }
    }

    // ------------------------------------------------------------ confirm

    fn ui_confirm(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let targets = self.tree.targets();
        let (count, bytes) = self.tree.selected_summary();
        let mut by_cat: Vec<(Category, u64)> = self.tree.selected_by_category().into_iter().collect();
        by_cat.sort_by(|a, b| b.1.cmp(&a.1));
        let has_review = targets
            .iter()
            .filter_map(|id| self.tree.node(*id))
            .any(|n| n.risk == Risk::Review && !n.is_vhd());
        let disks: Vec<ScanNode> = self
            .tree
            .selected_vhds()
            .iter()
            .filter_map(|id| self.tree.node(*id).cloned())
            .collect();

        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            theme::centered_column(ui, 640.0, |ui| {
                ui.add_space(16.0);
                ui.label(RichText::new(i18n::confirm_title()).font(display(28.0)));
                ui.label(RichText::new(i18n::confirm_body(count, &format_bytes(bytes))).color(p.muted));
                ui.add_space(14.0);

                if bytes > 0 || disks.is_empty() {
                theme::card(p).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(i18n::space_to_free()).color(p.muted).small());
                    ui.label(RichText::new(format_bytes(bytes)).font(display(44.0)).color(p.accent_ink()));
                    ui.add_space(10.0);
                    let max = by_cat.first().map(|c| c.1).unwrap_or(1).max(1);
                    for (cat, b) in &by_cat {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                            ui.painter().circle_filled(r.center(), 4.5, theme::category_color(*cat));
                            let (name_r, _) = ui.allocate_exact_size(Vec2::new(132.0, 18.0), Sense::hover());
                            ui.painter().text(
                                name_r.left_center(),
                                egui::Align2::LEFT_CENTER,
                                i18n::category_name(*cat),
                                body(13.5),
                                p.text,
                            );
                            let size_w = 80.0;
                            let bar_w = (ui.available_width() - size_w - 8.0).max(40.0);
                            let (bar, _) = ui.allocate_exact_size(Vec2::new(bar_w, 8.0), Sense::hover());
                            ui.painter().rect_filled(bar, CornerRadius::same(4), p.raised);
                            let mut f = bar;
                            f.set_width((bar_w * (*b as f32 / max as f32)).max(4.0));
                            ui.painter().rect_filled(f, CornerRadius::same(4), theme::category_color(*cat));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new(format_bytes(*b)).font(display(14.0)));
                            });
                        });
                    }
                });
                }

                if !disks.is_empty() {
                    ui.add_space(10.0);
                    theme::card(p).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(i18n::disks_to_compact()).font(strong(14.0)));
                        ui.add_space(4.0);
                        for d in &disks {
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                                ui.painter().circle_filled(r.center(), 4.5, theme::category_color(d.category));
                                ui.label(d.label.to_string()).on_hover_text(&d.detail);
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(RichText::new(format_bytes(d.size_bytes)).color(p.muted));
                                });
                            });
                        }
                        ui.add_space(6.0);
                        let (fast, slow) = cleaner_core::compact_estimate_all(disks.iter().map(|d| d.size_bytes));
                        ui.label(RichText::new(i18n::compact_estimate(fast, slow)).font(strong(14.0)).color(p.text));
                        ui.add_space(4.0);
                        egui::Frame::new()
                            .fill(mix(p.review, p.bg, 0.87))
                            .stroke(Stroke::new(1.0_f32, mix(p.review, p.bg, 0.5)))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(RichText::new(i18n::compact_slow_note()).color(p.review));
                                ui.add_space(4.0);
                                ui.label(RichText::new(i18n::compact_warning()).color(p.review));
                            });
                    });
                }

                if has_review {
                    ui.add_space(10.0);
                    egui::Frame::new()
                        .fill(mix(p.review, p.bg, 0.87))
                        .stroke(Stroke::new(1.0_f32, mix(p.review, p.bg, 0.5)))
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                RichText::new(i18n::review_warning())
                                .color(p.review),
                            );
                        });
                }

                ui.add_space(10.0);
                egui::CollapsingHeader::new(RichText::new(i18n::see_items(count)).color(p.muted))
                    .id_salt("confirm_items")
                    .show(ui, |ui| {
                        let mut nodes: Vec<&ScanNode> = targets.iter().filter_map(|id| self.tree.node(*id)).collect();
                        nodes.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
                        egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                            for n in nodes {
                                ui.horizontal(|ui| {
                                    let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                                    ui.painter().circle_filled(r.center(), 3.5, theme::category_color(n.category));
                                    ui.label(n.label.to_string()).on_hover_text(&n.detail);
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(RichText::new(format_bytes(n.size_bytes)).color(p.muted));
                                    });
                                });
                            }
                        });
                    });

                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    if theme::ghost_button(ui, p, &format!("  {}  ", i18n::back_and_review())).clicked() {
                        self.screen = Screen::Results;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let label = match (bytes > 0, disks.len()) {
                            (_, 0) => i18n::delete_size(&format_bytes(bytes)),
                            (false, n) => i18n::compact_disks(n),
                            (true, _) => i18n::delete_and_compact(&format_bytes(bytes)),
                        };
                        if theme::danger_button(ui, p, &format!("  {label}  ")).clicked() {
                            self.start_clean();
                        }
                    });
                });
            });
        });
    }

    // ------------------------------------------------------------ cleaning

    fn ui_cleaning(&mut self, ui: &mut egui::Ui, p: &Palette) {
        if let Some((c, at)) = self.compact.clone() {
            self.ui_compacting(ui, p, &c, at);
            return;
        }
        let (done, total, label) = self.delete_progress.clone();
        let frac = if total > 0 { done as f32 / total as f32 } else { 0.0 };
        let shown = ui.ctx().animate_value_with_time(egui::Id::new("clean_progress"), frac, 0.25);
        ui.add_space((ui.available_height() * 0.22).max(20.0));
        theme::centered_column(ui, 560.0, |ui| {
            ui.label(RichText::new(i18n::cleaning()).font(strong(16.0)).color(p.muted));
            ui.label(RichText::new(format!("{:.0}%", shown * 100.0)).font(display(72.0)).color(p.text));
            ui.add_space(6.0);
            theme::progress_bar(ui, p, shown, 10.0);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(i18n::done_of(done, total)).color(p.muted));
                if !label.is_empty() {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let w = ui.available_width();
                        let g = theme::truncated(ui, &label, FontIdExt::mono(12.0), p.muted, w);
                        ui.label(g);
                    });
                }
            });
        });
    }

    fn ui_compacting(&mut self, ui: &mut egui::Ui, p: &Palette, c: &CompactProgress, at: std::time::Instant) {
        let elapsed = c.elapsed_secs + at.elapsed().as_secs();
        let (fast, slow) = c.estimate_secs;
        // Real progress when diskpart's output is readable; otherwise the bar
        // follows the clock against the slow estimate and never fills up.
        let frac = match c.percent {
            Some(pct) => (c.disk as f32 + pct as f32 / 100.0) / c.disks.max(1) as f32,
            None => (elapsed as f32 / slow.max(1) as f32).min(0.95),
        };
        let shown = ui.ctx().animate_value_with_time(egui::Id::new("compact_progress"), frac, 0.4);
        ui.add_space((ui.available_height() * 0.16).max(20.0));
        theme::centered_column(ui, 600.0, |ui| {
            ui.label(RichText::new(i18n::compacting_title()).font(strong(16.0)).color(p.muted));
            let big = match c.percent {
                Some(_) => format!("{:.0}%", shown * 100.0),
                None => i18n::duration(elapsed),
            };
            ui.label(RichText::new(big).font(display(64.0)).color(p.text));
            ui.add_space(6.0);
            theme::progress_bar(ui, p, shown, 10.0);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(i18n::compacting_disk(c.disk + 1, c.disks)).color(p.text));
                ui.label(RichText::new(c.label.to_string()).color(p.muted));
            });
            ui.horizontal(|ui| {
                if c.percent.is_some() {
                    ui.label(RichText::new(i18n::elapsed(&i18n::duration(elapsed))).color(p.muted));
                    ui.add_space(8.0);
                }
                ui.label(RichText::new(i18n::compact_estimate(fast, slow)).color(p.muted));
            });
            if c.percent.is_none() && elapsed < 30 {
                ui.label(RichText::new(i18n::waiting_admin()).color(p.muted).small());
            }
            ui.add_space(14.0);
            egui::Frame::new()
                .fill(mix(p.review, p.bg, 0.87))
                .stroke(Stroke::new(1.0_f32, mix(p.review, p.bg, 0.5)))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let note = if elapsed > slow { i18n::past_estimate() } else { i18n::compact_slow_note() };
                    ui.label(RichText::new(note).color(p.review));
                });
        });
    }

    // ------------------------------------------------------------ report

    fn ui_report(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let freed: u64 = self
            .delete_results
            .iter()
            .filter(|r| r.outcome == DeleteOutcome::Removed)
            .map(|r| r.bytes_freed)
            .sum();
        let count = |o: DeleteOutcome| self.delete_results.iter().filter(|r| r.outcome == o).count();
        let (removed, skipped, failed) = (
            count(DeleteOutcome::Removed),
            count(DeleteOutcome::Skipped),
            count(DeleteOutcome::Failed),
        );
        // The freed number counts up once when the report opens.
        let shown = ui
            .ctx()
            .animate_value_with_time(egui::Id::new("freed_counter"), freed as f32, 1.1);

        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            theme::centered_column(ui, 640.0, |ui| {
                ui.add_space(24.0);
                ui.label(RichText::new(i18n::report_title()).font(strong(16.0)).color(p.muted));
                ui.label(RichText::new(format_bytes(shown as u64)).font(display(68.0)).color(p.safe));
                ui.label(RichText::new(i18n::freed_on_disk()).color(p.muted));
                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    let stat = |ui: &mut egui::Ui, n: usize, label: &str, color: Color32| {
                        theme::card(p).inner_margin(egui::Margin::symmetric(16, 10)).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(n.to_string()).font(display(22.0)).color(color));
                                ui.label(RichText::new(label).color(p.muted));
                            });
                        });
                    };
                    stat(ui, removed, i18n::stat_removed(), p.safe);
                    if skipped > 0 {
                        stat(ui, skipped, i18n::stat_skipped(), p.review);
                    }
                    if failed > 0 {
                        stat(ui, failed, i18n::stat_failed(), p.danger);
                    }
                });

                ui.add_space(14.0);
                theme::card(p).inner_margin(egui::Margin::symmetric(16, 10)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let mut items: Vec<&DeleteResultItem> = self.delete_results.iter().collect();
                    // Problems first, then biggest wins.
                    items.sort_by_key(|r| {
                        (
                            match r.outcome {
                                DeleteOutcome::Failed => 0,
                                DeleteOutcome::Skipped => 1,
                                DeleteOutcome::Removed => 2,
                            },
                            std::cmp::Reverse(r.bytes_freed),
                        )
                    });
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for item in items {
                            let color = match item.outcome {
                                DeleteOutcome::Removed => p.safe,
                                DeleteOutcome::Skipped => p.review,
                                DeleteOutcome::Failed => p.danger,
                            };
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(Vec2::new(8.0, 18.0), Sense::hover());
                                ui.painter().circle_filled(r.center(), 4.0, color);
                                ui.label(RichText::new(i18n::outcome_label(item.outcome)).color(color));
                                ui.label(item.label.to_string());
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if item.bytes_freed > 0 {
                                        ui.label(RichText::new(format_bytes(item.bytes_freed)).color(p.muted));
                                    }
                                });
                            });
                            if let Some(m) = &item.message {
                                ui.horizontal(|ui| {
                                    ui.add_space(16.0);
                                    ui.label(RichText::new(m.to_string()).color(p.muted).small());
                                });
                            }
                        }
                    });
                });

                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    if theme::ghost_button(ui, p, &format!("  {}  ", i18n::back_home())).clicked() {
                        self.tree.clear();
                        self.has_scanned = false;
                        self.screen = Screen::Home;
                    }
                    if theme::primary_button(ui, p, &format!("  {}  ", i18n::new_scan()), true).clicked() {
                        self.start_scan();
                    }
                });
            });
        });
    }
}

/// Small helper so the cleaning screen can ask for a monospace font id.
struct FontIdExt;
impl FontIdExt {
    fn mono(size: f32) -> egui::FontId {
        egui::FontId::monospace(size)
    }
}

impl eframe::App for CleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let p = self.palette();
        theme::apply(ctx, &p);
        self.poll_scan();
        self.poll_expand();
        self.poll_delete();
        if self.scanning || self.cleaning {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        self.ui_top_bar(ctx, &p);
        if self.screen == Screen::Results {
            self.ui_results_footer(ctx, &p);
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(egui::Margin { left: PAGE_MARGIN, right: PAGE_MARGIN, top: 6, bottom: 12 }),
            )
            .show(ctx, |ui| match self.screen {
                Screen::Home => self.ui_home(ui, &p),
                Screen::Results => self.ui_results(ui, &p),
                Screen::Confirm => self.ui_confirm(ui, &p),
                Screen::Cleaning => self.ui_cleaning(ui, &p),
                Screen::Report => self.ui_report(ui, &p),
            });
    }
}
