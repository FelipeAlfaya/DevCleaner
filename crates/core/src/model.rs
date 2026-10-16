use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::i18n::Text;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Category {
    Npm,
    Yarn,
    Pnpm,
    Bun,
    Pip,
    Uv,
    Cargo,
    Gradle,
    Maven,
    Go,
    NuGet,
    Composer,
    Docker,
    Wsl,
    Ide,
    Temp,
}

impl Category {
    pub fn all() -> &'static [Category] {
        &[
            Category::Npm,
            Category::Yarn,
            Category::Pnpm,
            Category::Bun,
            Category::Pip,
            Category::Uv,
            Category::Cargo,
            Category::Gradle,
            Category::Maven,
            Category::Go,
            Category::NuGet,
            Category::Composer,
            Category::Docker,
            Category::Wsl,
            Category::Ide,
            Category::Temp,
        ]
    }

    pub fn id(self) -> &'static str {
        match self {
            Category::Npm => "npm",
            Category::Yarn => "yarn",
            Category::Pnpm => "pnpm",
            Category::Bun => "bun",
            Category::Pip => "pip",
            Category::Uv => "uv",
            Category::Cargo => "cargo",
            Category::Gradle => "gradle",
            Category::Maven => "maven",
            Category::Go => "go",
            Category::NuGet => "nuget",
            Category::Composer => "composer",
            Category::Docker => "docker",
            Category::Wsl => "wsl",
            Category::Ide => "ide",
            Category::Temp => "temp",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Risk {
    Safe,
    Review,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeleteStrategy {
    FileSystem,
    NpmCacheClean,
    PnpmStorePrune,
    GoCleanCache,
    GoCleanModCache,
    DockerImage,
    DockerContainer,
    DockerVolume,
    DockerBuildCache,
    /// Shrinks a WSL/Docker virtual disk (VHDX) so Windows gets back the
    /// space freed inside it. Nothing is deleted.
    CompactVhd,
}

#[derive(Debug, Clone)]
pub enum NodeTarget {
    Path {
        path: PathBuf,
        jail_root: PathBuf,
    },
    DockerId {
        id: String,
        strategy: DeleteStrategy,
    },
    CategoryAction {
        category: Category,
        strategy: DeleteStrategy,
    },
    /// A WSL or Docker Desktop virtual disk. Selecting it compacts the file.
    Vhd {
        path: PathBuf,
    },
}

impl ScanNode {
    /// Virtual disks are compacted, not deleted: their size is not space
    /// that a cleanup frees, so totals leave them out.
    pub fn is_vhd(&self) -> bool {
        matches!(self.target, NodeTarget::Vhd { .. })
    }
}

#[derive(Debug, Clone)]
pub struct ScanNode {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub category: Category,
    pub label: Text,
    pub detail: String,
    pub size_bytes: u64,
    pub modified: Option<DateTime<Utc>>,
    pub reason: Text,
    pub risk: Risk,
    pub default_selected: bool,
    pub expandable: bool,
    pub children_loaded: bool,
    pub target: NodeTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    Unchecked,
    Checked,
    Partial,
}

#[derive(Debug, Clone)]
pub struct CategoryStatus {
    pub category: Category,
    pub available: bool,
    pub unavailable_reason: Option<Text>,
    pub total_bytes: u64,
    pub item_count: usize,
    /// Size of virtual disks found (compacted, not deleted).
    pub disk_bytes: u64,
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    CategoryStatus(CategoryStatus),
    NodesBatch(Vec<ScanNode>),
    CategoryFinished {
        category: Category,
    },
    ScanFinished,
    ScanError {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
    Removed,
    Skipped,
    Failed,
}

#[derive(Debug, Clone)]
pub struct DeleteResultItem {
    pub node_id: u64,
    pub label: Text,
    pub outcome: DeleteOutcome,
    pub message: Option<Text>,
    pub bytes_freed: u64,
}

#[derive(Debug, Clone)]
pub struct CompactProgress {
    /// Index of the disk being compacted (0-based).
    pub disk: usize,
    pub disks: usize,
    pub label: Text,
    pub elapsed_secs: u64,
    /// Rough (fastest, slowest) total time for every selected disk.
    pub estimate_secs: (u64, u64),
    /// Percentage reported by diskpart for the current disk, when readable.
    pub percent: Option<u8>,
}

#[derive(Debug, Clone)]
pub enum DeleteEvent {
    Progress {
        done: usize,
        total: usize,
        current_label: Text,
    },
    /// Sent about twice a second while virtual disks are being compacted.
    Compacting(CompactProgress),
    Item(DeleteResultItem),
    Finished,
}
