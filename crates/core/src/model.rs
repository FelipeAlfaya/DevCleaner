use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    DockerImage,
    DockerContainer,
    DockerVolume,
    DockerBuildCache,
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
}

#[derive(Debug, Clone)]
pub struct ScanNode {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub category: Category,
    pub label: String,
    pub detail: String,
    pub size_bytes: u64,
    pub modified: Option<DateTime<Utc>>,
    pub reason: String,
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
    pub unavailable_reason: Option<String>,
    pub total_bytes: u64,
    pub item_count: usize,
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
    pub label: String,
    pub outcome: DeleteOutcome,
    pub message: Option<String>,
    pub bytes_freed: u64,
}

#[derive(Debug, Clone)]
pub enum DeleteEvent {
    Progress {
        done: usize,
        total: usize,
        current_label: String,
    },
    Item(DeleteResultItem),
    Finished,
}
