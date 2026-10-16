//! Scan profiles: named sets of categories, so a scan can focus on one
//! ecosystem instead of checking everything.

use serde::{Deserialize, Serialize};

use crate::model::Category;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanProfile {
    /// Every category.
    #[default]
    Full,
    /// Package manager caches only (no Docker, WSL, IDEs or temp files).
    PackageManagers,
    JavaScript,
    Python,
    Jvm,
    RustGo,
    /// Docker and WSL virtual disks.
    Containers,
    /// Caches of code editors and AI apps.
    Editors,
    /// Categories picked by the user.
    Custom,
}

impl ScanProfile {
    pub fn all() -> &'static [ScanProfile] {
        &[
            ScanProfile::Full,
            ScanProfile::PackageManagers,
            ScanProfile::JavaScript,
            ScanProfile::Python,
            ScanProfile::Jvm,
            ScanProfile::RustGo,
            ScanProfile::Containers,
            ScanProfile::Editors,
            ScanProfile::Custom,
        ]
    }

    /// Categories of a built-in profile. `Custom` returns `custom`.
    pub fn categories(self, custom: &[Category]) -> Vec<Category> {
        use Category::*;
        let list: &[Category] = match self {
            ScanProfile::Full => Category::all(),
            ScanProfile::PackageManagers => &[
                Npm, Yarn, Pnpm, Bun, Pip, Uv, Cargo, Gradle, Maven, Go, NuGet, Composer,
            ],
            ScanProfile::JavaScript => &[Npm, Yarn, Pnpm, Bun],
            ScanProfile::Python => &[Pip, Uv],
            ScanProfile::Jvm => &[Gradle, Maven],
            ScanProfile::RustGo => &[Cargo, Go],
            ScanProfile::Containers => &[Docker, Wsl],
            ScanProfile::Editors => &[Ide],
            ScanProfile::Custom => {
                // Keep the canonical order and drop duplicates.
                return Category::all()
                    .iter()
                    .copied()
                    .filter(|c| custom.contains(c))
                    .collect();
            }
        };
        list.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_keeps_order_and_dedupes() {
        let got = ScanProfile::Custom.categories(&[Category::Temp, Category::Npm, Category::Npm]);
        assert_eq!(got, vec![Category::Npm, Category::Temp]);
    }

    #[test]
    fn full_has_everything() {
        assert_eq!(ScanProfile::Full.categories(&[]).len(), Category::all().len());
    }
}
