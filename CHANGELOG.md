# Changelog

All notable changes to DevCleaner are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.2.0 - 2026-10-16

### Added

- **New interface.** A palette of its own with light and dark themes, a colored bar showing the space each category takes, a layered illustration on the home screen built from the last scan, and redrawn checkboxes, filters, risk labels and size bars.
- **Results screen.**
  - Items are sorted from largest to smallest by default. They can also be sorted from smallest to largest, by name (A–Z or Z–A), by category or by risk.
  - Filters can be combined: several categories, risk level, text search (which also matches items inside folders) and "Selected only".
  - "Select visible", "Unselect visible" and "Clear filters" buttons.
  - A footer that always shows the exact amount that will be deleted.
- **Confirmation screen** with the space each category will free, a warning when items marked Review are selected, and the full list of items.
- **Report screen** with the space freed and how many items were removed, skipped or failed.
- **Languages.** English, Portuguese (pt-BR) and Spanish. The app follows the system language on first run, can be switched at any time from the top bar or Preferences, and shows sizes with the right decimal separator.
- **IDEs and AI apps category.** Caches of Cursor, VS Code, VS Code Insiders, VSCodium, Windsurf, Antigravity, Trae, Kiro and the Claude desktop app: browser and GPU caches, compiled code caches, downloaded extension installers, logs, crash reports and pending update installers.
  - Per-workspace state (which in Cursor includes AI chat history), Cursor's local code index and Claude Code's conversation history are marked Review.
  - When `workspaceStorage` is opened, each entry shows the project it belongs to instead of a hash.
  - `globalStorage`, the local file history, settings and installed extensions are never listed.
- **WSL category (Windows).** Lists the virtual disk of each WSL distro, and dev caches (pip, uv, Go, Yarn, pnpm, npm, Bun, Cargo, Gradle, Maven, and the VS Code and Cursor servers) inside the home folders of distros that are already running. A scan never starts a stopped distro.
- **Virtual disk compaction.** Selecting a WSL or Docker Desktop disk compacts it with `diskpart` so Windows gets back the space freed inside it. Nothing is deleted. WSL is shut down first, every selected disk is handled in one run, and Windows asks for administrator permission once. The report shows the space actually reclaimed. The confirmation screen shows an estimated time and warns that it can take long; while it runs, the progress reported by `diskpart` (or the elapsed time) is shown.
- **Docker.**
  - The Docker Desktop virtual disk is listed even when Docker is closed, and the custom disk location set in Docker Desktop is respected.
  - Tagged images that no container uses are listed, marked Review.
- **Scan profiles.** Everything, Package managers, JavaScript, Python, Java / Kotlin, Rust / Go, Docker and WSL, IDEs and AI, and Custom. In Custom, categories are picked by clicking them on the home screen. The profile is saved in Preferences.
- README, MIT license and this changelog.

### Changed

- Items marked Review always start unselected, whatever the scanner asks for.
- Stopped Docker containers are now marked Review and start unselected, because `docker rm` discards anything written inside them outside volumes.
- The Go module cache is cleared with `go clean -modcache` instead of deleting folders, since Go writes it read-only.
- The Go caches are located with `go env GOCACHE` and `go env GOMODCACHE`.
- The app is now called DevCleaner.
- Buttons and filters grow slightly when hovered and shrink when pressed, and everything clickable shows a hand cursor.
- The project is licensed under MIT only.
- Release builds on Windows open without a console window.

### Fixed

- Sizes were counted twice when a folder was opened: both the folder and the items inside it were added to the total.
- Unchecking every item inside a folder left the folder selected internally, so the whole folder would still be deleted.
- Items loaded after a folder was checked or unchecked did not follow the folder's state.
- The Go build cache was not found on Windows (`%LocalAppData%\go-build`).
- Temporary files were shown as folders that could be opened.
- After a cleanup, the old results stayed available even though their items had been deleted.
- External tools (`docker`, `npm`, `go` and others) flashed a console window on Windows.

## 0.1.0

- First version: scans the caches of npm, Yarn, pnpm, Bun, pip, uv, Cargo, Gradle, Maven, Go, NuGet, Composer, Docker and temporary files, and deletes the selected items after confirmation.

