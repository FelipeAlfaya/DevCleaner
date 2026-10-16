//! Application state and the logic shared by every screen. Rendering lives
//! in `widgets.rs` (building blocks) and `screens/` (one file per screen).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cleaner_core::scanners;
use cleaner_core::{
    start_delete, start_scan, Category, CompactProgress, DeleteEvent, DeleteHandle,
    DeleteResultItem, Lang, ScanEvent, ScanHandle, ScanNode, ScanProfile, ScanTree, Settings,
    ViewFilter,
};
use gpui::{
    div, point, prelude::*, px, Context, FocusHandle, KeyDownEvent, MouseButton, MouseMoveEvent,
    MouseUpEvent, ScrollHandle, SharedString, UniformListScrollHandle, Window,
};

use crate::screens;
use crate::widgets::ScrollArea;
use crate::theme::{self, Palette};

/// One hover transition: animates from `from` towards 1 (hovered) or 0.
#[derive(Clone, Copy)]
struct HoverAnim {
    on: bool,
    from: f32,
    at: Instant,
}

const HOVER_DURATION: f32 = 0.2;

impl HoverAnim {
    fn value(&self) -> (f32, bool) {
        let p = (self.at.elapsed().as_secs_f32() / HOVER_DURATION).min(1.0);
        // Ease-in-out, like CSS `transition: opacity .2s ease`.
        let eased = p * p * (3.0 - 2.0 * p);
        let target = if self.on { 1.0 } else { 0.0 };
        (self.from + (target - self.from) * eased, p < 1.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Results,
    Confirm,
    Cleaning,
    Report,
}

/// Popup menus; only one is open at a time.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    Sort,
    Language,
}

pub struct DevCleaner {
    pub settings: Settings,
    pub screen: Screen,
    pub tree: ScanTree,
    scan_handle: Option<ScanHandle>,
    delete_handle: Option<DeleteHandle>,
    pub delete_results: Vec<DeleteResultItem>,
    pub delete_progress: (usize, usize, String),
    pub compact: Option<(CompactProgress, Instant)>,
    pub filter: ViewFilter,
    pub scanning: bool,
    pub cleaning: bool,
    pub categories_done: usize,
    pub scan_total: usize,
    pub scanned_profile: ScanProfile,
    pub has_scanned: bool,
    expand_tx: crossbeam_channel::Sender<Vec<ScanNode>>,
    expand_rx: crossbeam_channel::Receiver<Vec<ScanNode>>,
    pub search_focus: FocusHandle,
    pub menu: Option<Menu>,
    pub show_confirm_items: bool,
    pub report_opened: Option<Instant>,
    pub list_scroll: UniformListScrollHandle,
    pub home_scroll: ScrollHandle,
    pub confirm_scroll: ScrollHandle,
    pub report_scroll: ScrollHandle,
    /// Scrollbar being dragged: (area, pointer y at start, scroll offset at start).
    pub scroll_drag: Option<(ScrollArea, f32, f32)>,
    /// Hover transition per element id (buttons, chips, tiles).
    hover: HashMap<SharedString, HoverAnim>,
    /// When the home illustration started its intro (set on first paint,
    /// cleared when coming back home so it plays again).
    pub intro_at: Option<Instant>,
}

impl DevCleaner {
    pub fn new(settings: Settings, cx: &mut Context<Self>) -> Self {
        cleaner_core::i18n::set_lang(settings.effective_language());
        let (expand_tx, expand_rx) = crossbeam_channel::unbounded();

        // Background work reports through channels; poll them a few times a
        // second and redraw only when something changed.
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_millis(100)).await;
            if this.update(cx, |app, cx| app.tick(cx)).is_err() {
                break;
            }
        })
        .detach();

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
            search_focus: cx.focus_handle(),
            menu: None,
            show_confirm_items: false,
            report_opened: None,
            list_scroll: UniformListScrollHandle::new(),
            home_scroll: ScrollHandle::new(),
            confirm_scroll: ScrollHandle::new(),
            report_scroll: ScrollHandle::new(),
            scroll_drag: None,
            hover: HashMap::new(),
            intro_at: None,
        }
    }

    pub fn scroll_handle(&self, area: ScrollArea) -> ScrollHandle {
        match area {
            ScrollArea::Home => self.home_scroll.clone(),
            ScrollArea::Results => self.list_scroll.0.borrow().base_handle.clone(),
            ScrollArea::Confirm => self.confirm_scroll.clone(),
            ScrollArea::Report => self.report_scroll.clone(),
        }
    }

    fn on_drag_scroll(&mut self, ev: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some((area, y0, start)) = self.scroll_drag else { return };
        if ev.pressed_button != Some(MouseButton::Left) {
            self.scroll_drag = None;
            cx.notify();
            return;
        }
        let handle = self.scroll_handle(area);
        if let Some((_, view, _, thumb, max)) = crate::widgets::thumb_geometry(&handle) {
            let per_px = max / (view - thumb).max(1.0);
            let target = (start + (f32::from(ev.position.y) - y0) * per_px).clamp(0.0, max);
            handle.set_offset(point(handle.offset().x, px(-target)));
            cx.notify();
        }
    }

    pub fn palette(&self) -> Palette {
        Palette::new(self.settings.dark_theme)
    }

    // ------------------------------------------------------------ workers

    fn tick(&mut self, cx: &mut Context<Self>) {
        let mut changed = self.poll_scan();
        changed |= self.poll_expand();
        changed |= self.poll_delete();
        // The compaction clock ticks even without new events.
        if changed || self.compact.is_some() && self.cleaning {
            cx.notify();
        }
    }

    pub fn start_scan(&mut self) {
        if let Some(h) = self.scan_handle.take() {
            h.cancel();
        }
        if self.settings.scan_categories().is_empty() {
            return;
        }
        self.tree.clear();
        self.categories_done = 0;
        self.scan_total = self.settings.scan_categories().len();
        self.scanned_profile = self.settings.profile;
        self.scanning = true;
        self.has_scanned = true;
        self.screen = Screen::Results;
        self.menu = None;
        self.scan_handle = Some(start_scan(self.settings.clone()));
    }

    pub fn stop_scan(&mut self) {
        if let Some(h) = self.scan_handle.take() {
            h.cancel();
        }
        self.scanning = false;
    }

    fn poll_scan(&mut self) -> bool {
        let mut changed = false;
        let mut finished = false;
        if let Some(handle) = &self.scan_handle {
            while let Ok(event) = handle.events.try_recv() {
                changed = true;
                match event {
                    ScanEvent::CategoryStatus(s) => self.tree.set_category_status(s),
                    ScanEvent::NodesBatch(batch) => self.tree.ingest_nodes(batch),
                    ScanEvent::CategoryFinished { .. } => self.categories_done += 1,
                    ScanEvent::ScanFinished => finished = true,
                    ScanEvent::ScanError { message } => {
                        eprintln!("scan error: {message}");
                        finished = true;
                    }
                }
            }
        }
        if finished {
            self.scanning = false;
            self.scan_handle = None;
        }
        changed
    }

    fn poll_expand(&mut self) -> bool {
        let mut changed = false;
        while let Ok(batch) = self.expand_rx.try_recv() {
            self.tree.ingest_nodes(batch);
            changed = true;
        }
        changed
    }

    pub fn toggle_expand(&mut self, id: u64) {
        if self.tree.expanded.contains(&id) {
            self.tree.expanded.remove(&id);
            return;
        }
        let Some(node) = self.tree.node(id).cloned() else { return };
        self.tree.expanded.insert(id);
        if self.tree.children_loaded.contains(&id) {
            return;
        }
        self.tree.children_loaded.insert(id);
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

    pub fn start_clean(&mut self) {
        let targets = self.tree.selected_ids();
        let nodes = self.tree.nodes.clone();
        self.delete_results.clear();
        self.delete_progress = (0, targets.len(), String::new());
        self.compact = None;
        self.cleaning = true;
        self.screen = Screen::Cleaning;
        self.delete_handle = Some(start_delete(nodes, targets));
    }

    fn poll_delete(&mut self) -> bool {
        let mut changed = false;
        let mut finished = false;
        if let Some(handle) = &self.delete_handle {
            while let Ok(ev) = handle.events.try_recv() {
                changed = true;
                match ev {
                    DeleteEvent::Progress { done, total, current_label } => {
                        self.delete_progress = (done, total, current_label.to_string())
                    }
                    DeleteEvent::Item(item) => self.delete_results.push(item),
                    DeleteEvent::Compacting(c) => self.compact = Some((c, Instant::now())),
                    DeleteEvent::Finished => finished = true,
                }
            }
        }
        if finished {
            self.cleaning = false;
            self.delete_handle = None;
            // The scan is out of date now; the report keeps its own list.
            self.tree.clear();
            self.has_scanned = false;
            self.report_opened = Some(Instant::now());
            self.screen = Screen::Report;
        }
        changed
    }

    pub fn save_settings(&self) {
        let _ = self.settings.save();
    }

    pub fn set_language(&mut self, lang: Lang) {
        self.settings.language = Some(lang);
        cleaner_core::i18n::set_lang(lang);
        self.save_settings();
    }

    pub fn clear_filters(&mut self) {
        let sort = self.filter.sort;
        self.filter = ViewFilter { sort, ..Default::default() };
    }

    pub fn go_home(&mut self) {
        if self.screen != Screen::Home {
            self.intro_at = None;
        }
        self.screen = Screen::Home;
        self.menu = None;
    }

    // ------------------------------------------------------------ hover

    pub fn set_hover(&mut self, id: &SharedString, on: bool) {
        let current = self.hover.get(id).map(|h| (h.on, h.value().0));
        match current {
            Some((was_on, _)) if was_on == on => {}
            // Start from wherever the previous transition is, so leaving
            // halfway reverses smoothly instead of jumping.
            Some((_, value)) => {
                self.hover.insert(id.clone(), HoverAnim { on, from: value, at: Instant::now() });
            }
            None if on => {
                self.hover.insert(id.clone(), HoverAnim { on, from: 0.0, at: Instant::now() });
            }
            None => {}
        }
    }

    /// Hover progress in 0..=1. Keeps requesting frames while it animates.
    pub fn hover_t(&self, id: &str, window: &Window) -> f32 {
        let Some(h) = self.hover.get(id) else { return 0.0 };
        let (value, running) = h.value();
        if running {
            window.request_animation_frame();
        }
        value
    }

    // ------------------------------------------------------------ data helpers

    /// Categories ordered by bytes found, heaviest first.
    pub fn strata_segments(&self) -> Vec<(Category, u64, u64)> {
        let found = self.tree.found_by_category();
        let selected = self.tree.selected_by_category();
        let mut segs: Vec<(Category, u64, u64)> = found
            .into_iter()
            .filter(|(_, b)| *b > 0)
            .map(|(c, f)| (c, f, selected.get(&c).copied().unwrap_or(0)))
            .collect();
        segs.sort_by(|a, b| b.1.cmp(&a.1));
        segs
    }

    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.search_focus.is_focused(window) {
            if ev.keystroke.key == "escape" && self.menu.is_some() {
                self.menu = None;
                cx.notify();
            }
            return;
        }
        match ev.keystroke.key.as_str() {
            "backspace" => {
                self.filter.search.pop();
            }
            "escape" => {
                if self.filter.search.is_empty() {
                    window.blur();
                } else {
                    self.filter.search.clear();
                }
            }
            "enter" | "tab" => window.blur(),
            _ => {
                let mods = &ev.keystroke.modifiers;
                if mods.control || mods.platform || mods.alt {
                    return;
                }
                if let Some(ch) = ev.keystroke.key_char.as_ref() {
                    self.filter.search.push_str(ch);
                }
            }
        }
        cx.notify();
    }
}

impl Render for DevCleaner {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
        let body = match self.screen {
            Screen::Home => screens::home::render(self, window, cx).into_any_element(),
            Screen::Results => screens::results::render(self, window, cx).into_any_element(),
            Screen::Confirm => screens::flow::confirm(self, window, cx).into_any_element(),
            Screen::Cleaning => screens::flow::cleaning(self, window, cx).into_any_element(),
            Screen::Report => screens::flow::report(self, window, cx).into_any_element(),
        };
        div()
            .id("root")
            .key_context("DevCleaner")
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_move(cx.listener(Self::on_drag_scroll))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, cx| {
                    if this.scroll_drag.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .size_full()
            .flex()
            .flex_col()
            .bg(p.bg)
            .text_color(p.text)
            .font_family(theme::body_font())
            .text_size(px(14.))
            .relative()
            .child(crate::widgets::top_bar(self, window, cx))
            .child(div().flex_1().min_h_0().flex().flex_col().child(body))
            .when(self.menu.is_some(), |d| d.child(crate::widgets::menu_backdrop(cx)))
    }
}
