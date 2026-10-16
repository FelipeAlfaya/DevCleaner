mod bun;
mod cargo;
mod common;
mod composer;
mod docker;
mod go;
mod gradle;
mod maven;
mod npm;
mod nuget;
mod pip;
mod pnpm;
mod temp;
mod uv;
mod yarn;

use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::Sender;

use crate::model::{Category, ScanEvent, ScanNode};
use crate::settings::Settings;

pub fn run_category(
    category: Category,
    settings: &Settings,
    tx: &Sender<ScanEvent>,
    cancel: &AtomicBool,
) {
    if cancel.load(Ordering::Relaxed) {
        return;
    }
    match category {
        Category::Npm => npm::scan(settings, tx, cancel),
        Category::Yarn => yarn::scan(settings, tx, cancel),
        Category::Pnpm => pnpm::scan(settings, tx, cancel),
        Category::Bun => bun::scan(settings, tx, cancel),
        Category::Pip => pip::scan(settings, tx, cancel),
        Category::Uv => uv::scan(settings, tx, cancel),
        Category::Cargo => cargo::scan(settings, tx, cancel),
        Category::Gradle => gradle::scan(settings, tx, cancel),
        Category::Maven => maven::scan(settings, tx, cancel),
        Category::Go => go::scan(settings, tx, cancel),
        Category::NuGet => nuget::scan(settings, tx, cancel),
        Category::Composer => composer::scan(settings, tx, cancel),
        Category::Docker => docker::scan(settings, tx, cancel),
        Category::Temp => temp::scan(settings, tx, cancel),
    }
}

pub fn expand_node(node: &ScanNode, settings: &Settings, tx: &Sender<ScanEvent>) {
    match node.category {
        Category::Temp => temp::expand(node, settings, tx),
        Category::Maven => maven::expand(node, settings, tx),
        Category::Gradle => gradle::expand(node, settings, tx),
        Category::Cargo => cargo::expand(node, settings, tx),
        _ => {}
    }
}
