use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::model::{Category, CategoryStatus, CheckState, Risk, ScanNode};

/// How the result list is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// Heaviest first (default).
    #[default]
    SizeDesc,
    SizeAsc,
    NameAsc,
    NameDesc,
    Category,
    Risk,
}

impl SortKey {
    pub fn all() -> &'static [SortKey] {
        &[
            SortKey::SizeDesc,
            SortKey::SizeAsc,
            SortKey::NameAsc,
            SortKey::NameDesc,
            SortKey::Category,
            SortKey::Risk,
        ]
    }

    pub fn compare(self, a: &ScanNode, b: &ScanNode) -> Ordering {
        let by_size_desc = || b.size_bytes.cmp(&a.size_bytes);
        let by_name = || a.label.to_string().to_lowercase().cmp(&b.label.to_string().to_lowercase());
        match self {
            SortKey::SizeDesc => by_size_desc().then_with(by_name),
            SortKey::SizeAsc => a.size_bytes.cmp(&b.size_bytes).then_with(by_name),
            SortKey::NameAsc => by_name(),
            SortKey::NameDesc => by_name().reverse(),
            SortKey::Category => category_index(a.category)
                .cmp(&category_index(b.category))
                .then_with(by_size_desc),
            SortKey::Risk => risk_index(a.risk)
                .cmp(&risk_index(b.risk))
                .then_with(by_size_desc),
        }
    }
}

fn category_index(c: Category) -> usize {
    Category::all().iter().position(|x| *x == c).unwrap_or(usize::MAX)
}

fn risk_index(r: Risk) -> u8 {
    match r {
        Risk::Safe => 0,
        Risk::Review => 1,
        Risk::Blocked => 2,
    }
}

/// Filters and ordering applied to the result list.
#[derive(Debug, Clone, Default)]
pub struct ViewFilter {
    pub search: String,
    /// Empty = every category.
    pub categories: HashSet<Category>,
    /// Empty = every risk level.
    pub risks: HashSet<Risk>,
    pub only_selected: bool,
    pub sort: SortKey,
}

impl ViewFilter {
    pub fn is_default(&self) -> bool {
        self.search.trim().is_empty()
            && self.categories.is_empty()
            && self.risks.is_empty()
            && !self.only_selected
    }
}

/// One line of the flattened, visible tree.
#[derive(Debug, Clone, Copy)]
pub struct VisibleRow {
    pub id: u64,
    pub depth: usize,
}

#[derive(Default)]
pub struct ScanTree {
    pub nodes: Vec<ScanNode>,
    pub by_id: HashMap<u64, usize>,
    pub category_status: HashMap<Category, CategoryStatus>,
    pub selection: HashMap<u64, bool>,
    pub expanded: HashSet<u64>,
    pub children_loaded: HashSet<u64>,
    children: HashMap<u64, Vec<u64>>,
}

impl ScanTree {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn ingest_nodes(&mut self, batch: Vec<ScanNode>) {
        for node in batch {
            if self.by_id.contains_key(&node.id) {
                continue;
            }
            // Children loaded after the parent was (un)checked inherit its state,
            // so expanding a folder never silently changes what will be deleted.
            let inherited = node
                .parent_id
                .and_then(|p| self.selection.get(&p).copied());
            // Only "Safe" items start checked; "Review" ones always wait for
            // the user, whatever the scanner asked for.
            let selected = match node.risk {
                Risk::Blocked => false,
                Risk::Review => inherited.unwrap_or(false),
                Risk::Safe => inherited.unwrap_or(node.default_selected),
            };
            self.selection.entry(node.id).or_insert(selected);
            if let Some(parent) = node.parent_id {
                self.children.entry(parent).or_default().push(node.id);
            }
            let idx = self.nodes.len();
            self.by_id.insert(node.id, idx);
            self.nodes.push(node);
        }
    }

    pub fn set_category_status(&mut self, status: CategoryStatus) {
        self.category_status.insert(status.category, status);
    }

    pub fn node(&self, id: u64) -> Option<&ScanNode> {
        self.by_id.get(&id).map(|&i| &self.nodes[i])
    }

    /// Checks or unchecks a node and its whole loaded subtree.
    pub fn toggle(&mut self, id: u64, checked: bool) {
        let Some(node) = self.node(id) else { return };
        if node.risk == Risk::Blocked {
            return;
        }
        self.selection.insert(id, checked);
        for child in self.child_ids(id) {
            self.toggle(child, checked);
        }
    }

    pub fn check_state(&self, id: u64) -> CheckState {
        let Some(node) = self.node(id) else {
            return CheckState::Unchecked;
        };
        if node.risk == Risk::Blocked {
            return CheckState::Unchecked;
        }
        let children = self.children.get(&id);
        let selectable: Vec<u64> = children
            .map(|c| {
                c.iter()
                    .copied()
                    .filter(|cid| self.node(*cid).is_some_and(|n| n.risk != Risk::Blocked))
                    .collect()
            })
            .unwrap_or_default();
        if children.is_none_or(|c| c.is_empty()) {
            return if self.is_selected(id) {
                CheckState::Checked
            } else {
                CheckState::Unchecked
            };
        }
        if selectable.is_empty() {
            return CheckState::Unchecked;
        }
        let mut any = false;
        let mut all = true;
        for cid in selectable {
            match self.check_state(cid) {
                CheckState::Checked => any = true,
                CheckState::Unchecked => all = false,
                CheckState::Partial => {
                    any = true;
                    all = false;
                }
            }
        }
        match (any, all) {
            (true, true) => CheckState::Checked,
            (true, false) => CheckState::Partial,
            _ => CheckState::Unchecked,
        }
    }

    pub fn is_selected(&self, id: u64) -> bool {
        self.selection.get(&id).copied().unwrap_or(false)
    }

    fn has_blocked_descendant(&self, id: u64) -> bool {
        self.child_ids(id).into_iter().any(|cid| {
            self.node(cid).is_some_and(|n| n.risk == Risk::Blocked)
                || self.has_blocked_descendant(cid)
        })
    }

    fn collect_targets(&self, id: u64, out: &mut Vec<u64>) {
        let Some(node) = self.node(id) else { return };
        if node.risk == Risk::Blocked {
            return;
        }
        let has_children = self.children.get(&id).is_some_and(|c| !c.is_empty());
        if !has_children {
            if self.is_selected(id) {
                out.push(id);
            }
            return;
        }
        match self.check_state(id) {
            CheckState::Checked if !self.has_blocked_descendant(id) => out.push(id),
            CheckState::Unchecked => {}
            _ => {
                for cid in self.child_ids(id) {
                    self.collect_targets(cid, out);
                }
            }
        }
    }

    /// The exact set of nodes that will be deleted. A fully checked folder is
    /// deleted as a whole; a partially checked one only through its checked
    /// children. A node and its descendants never appear together, so sizes
    /// summed over this set are never counted twice.
    pub fn targets(&self) -> Vec<u64> {
        let mut out = Vec::new();
        for root in self.root_nodes() {
            self.collect_targets(root.id, &mut out);
        }
        out
    }

    pub fn selected_ids(&self) -> HashSet<u64> {
        self.targets().into_iter().collect()
    }

    /// (items, bytes) that would be removed.
    pub fn selected_summary(&self) -> (usize, u64) {
        let targets = self.targets();
        let bytes = targets
            .iter()
            .filter_map(|id| self.node(*id))
            .filter(|n| !n.is_vhd())
            .map(|n| n.size_bytes)
            .sum();
        (targets.len(), bytes)
    }

    /// Selected virtual disks (compacted, not deleted).
    pub fn selected_vhds(&self) -> Vec<u64> {
        self.targets()
            .into_iter()
            .filter(|id| self.node(*id).is_some_and(|n| n.is_vhd()))
            .collect()
    }

    /// Bytes that would be removed, per category.
    pub fn selected_by_category(&self) -> HashMap<Category, u64> {
        let mut map = HashMap::new();
        for id in self.targets() {
            if let Some(n) = self.node(id).filter(|n| !n.is_vhd()) {
                *map.entry(n.category).or_insert(0) += n.size_bytes;
            }
        }
        map
    }

    /// Bytes found per category (top-level items only).
    pub fn found_by_category(&self) -> HashMap<Category, u64> {
        let mut map = HashMap::new();
        for n in self.root_nodes().into_iter().filter(|n| !n.is_vhd()) {
            *map.entry(n.category).or_insert(0) += n.size_bytes;
        }
        map
    }

    pub fn total_found(&self) -> u64 {
        self.root_nodes()
            .iter()
            .filter(|n| !n.is_vhd())
            .map(|n| n.size_bytes)
            .sum()
    }

    pub fn root_nodes(&self) -> Vec<&ScanNode> {
        self.nodes.iter().filter(|n| n.parent_id.is_none()).collect()
    }

    pub fn child_ids(&self, parent: u64) -> Vec<u64> {
        self.children.get(&parent).cloned().unwrap_or_default()
    }

    fn matches_search(&self, node: &ScanNode, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }
        let hit = |n: &ScanNode| {
            n.label.to_string().to_lowercase().contains(needle) || n.detail.to_lowercase().contains(needle)
        };
        hit(node)
            || self
                .child_ids(node.id)
                .iter()
                .filter_map(|c| self.node(*c))
                .any(|c| self.matches_search(c, needle))
    }

    pub fn visible_roots(&self, filter: &ViewFilter) -> Vec<&ScanNode> {
        let needle = filter.search.trim().to_lowercase();
        let mut roots: Vec<&ScanNode> = self
            .root_nodes()
            .into_iter()
            .filter(|n| filter.categories.is_empty() || filter.categories.contains(&n.category))
            .filter(|n| filter.risks.is_empty() || filter.risks.contains(&n.risk))
            .filter(|n| !filter.only_selected || self.check_state(n.id) != CheckState::Unchecked)
            .filter(|n| self.matches_search(n, &needle))
            .collect();
        roots.sort_by(|a, b| filter.sort.compare(a, b));
        roots
    }

    fn push_children(&self, id: u64, depth: usize, filter: &ViewFilter, out: &mut Vec<VisibleRow>) {
        if !self.expanded.contains(&id) {
            return;
        }
        let mut kids: Vec<&ScanNode> = self
            .child_ids(id)
            .iter()
            .filter_map(|c| self.node(*c))
            .filter(|n| !filter.only_selected || self.check_state(n.id) != CheckState::Unchecked)
            .collect();
        kids.sort_by(|a, b| filter.sort.compare(a, b));
        for k in kids {
            out.push(VisibleRow { id: k.id, depth });
            self.push_children(k.id, depth + 1, filter, out);
        }
    }

    /// Visible rows in display order, expanded children included.
    pub fn visible_rows(&self, filter: &ViewFilter) -> Vec<VisibleRow> {
        let mut out = Vec::new();
        for root in self.visible_roots(filter) {
            out.push(VisibleRow { id: root.id, depth: 0 });
            self.push_children(root.id, 1, filter, &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NodeTarget, DeleteStrategy};
    use crate::i18n::Text;

    fn node(id: u64, parent: Option<u64>, size: u64, label: &str) -> ScanNode {
        ScanNode {
            id,
            parent_id: parent,
            category: Category::Temp,
            label: label.into(),
            detail: String::new(),
            size_bytes: size,
            modified: None,
            reason: Text::Raw(String::new()),
            risk: Risk::Safe,
            default_selected: true,
            expandable: parent.is_none(),
            children_loaded: false,
            target: NodeTarget::CategoryAction {
                category: Category::Temp,
                strategy: DeleteStrategy::FileSystem,
            },
        }
    }

    #[test]
    fn expanded_parent_is_not_counted_twice() {
        let mut t = ScanTree::default();
        t.ingest_nodes(vec![node(1, None, 100, "p")]);
        t.ingest_nodes(vec![node(2, Some(1), 60, "a"), node(3, Some(1), 40, "b")]);
        assert_eq!(t.selected_summary(), (1, 100));
        t.toggle(2, false);
        assert_eq!(t.selected_summary(), (1, 40));
        t.toggle(3, false);
        assert_eq!(t.selected_summary(), (0, 0));
        assert_eq!(t.check_state(1), CheckState::Unchecked);
    }

    #[test]
    fn children_inherit_unchecked_parent() {
        let mut t = ScanTree::default();
        t.ingest_nodes(vec![node(1, None, 100, "p")]);
        t.toggle(1, false);
        t.ingest_nodes(vec![node(2, Some(1), 60, "a")]);
        assert!(!t.is_selected(2));
    }

    #[test]
    fn review_items_start_unchecked() {
        let mut t = ScanTree::default();
        let mut n = node(1, None, 100, "vol");
        n.risk = Risk::Review;
        n.default_selected = true;
        t.ingest_nodes(vec![n]);
        assert!(!t.is_selected(1));
        assert_eq!(t.selected_summary(), (0, 0));
    }

    #[test]
    fn virtual_disks_are_not_counted_as_deleted_space() {
        let mut t = ScanTree::default();
        let mut disk = node(1, None, 40_000, "docker_data.vhdx");
        disk.target = NodeTarget::Vhd { path: "docker_data.vhdx".into() };
        disk.risk = Risk::Review;
        t.ingest_nodes(vec![disk, node(2, None, 100, "cache")]);
        t.toggle(1, true);
        assert_eq!(t.selected_summary(), (2, 100));
        assert_eq!(t.selected_vhds(), vec![1]);
        assert_eq!(t.total_found(), 100);
    }

    #[test]
    fn default_sort_is_heaviest_first() {
        let mut t = ScanTree::default();
        t.ingest_nodes(vec![
            node(1, None, 10, "small"),
            node(2, None, 300, "big"),
            node(3, None, 50, "mid"),
        ]);
        let ids: Vec<u64> = t
            .visible_roots(&ViewFilter::default())
            .iter()
            .map(|n| n.id)
            .collect();
        assert_eq!(ids, vec![2, 3, 1]);
    }
}
