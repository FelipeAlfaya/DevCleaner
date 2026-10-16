use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::i18n::Lang;
use crate::model::Category;
use crate::profiles::ScanProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub temp_min_age_days: u32,
    pub dark_theme: bool,
    /// Interface language. `None` follows the operating system.
    #[serde(default)]
    pub language: Option<Lang>,
    /// Which categories a scan checks.
    #[serde(default)]
    pub profile: ScanProfile,
    /// Categories of the custom profile.
    #[serde(default = "all_categories")]
    pub custom_categories: Vec<Category>,
}

fn all_categories() -> Vec<Category> {
    Category::all().to_vec()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            temp_min_age_days: 7,
            dark_theme: true,
            language: None,
            profile: ScanProfile::Full,
            custom_categories: all_categories(),
        }
    }
}

impl Settings {
    /// Categories the current profile scans.
    pub fn scan_categories(&self) -> Vec<Category> {
        self.profile.categories(&self.custom_categories)
    }

    /// Language chosen by the user, or the system one.
    pub fn effective_language(&self) -> Lang {
        self.language.unwrap_or_else(Lang::detect)
    }

    pub fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("dev", "cleaner", "cleaner").map(|d| d.config_dir().join("settings.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::config_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }
}
