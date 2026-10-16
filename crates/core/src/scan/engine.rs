use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crossbeam_channel::Receiver;

use crate::model::ScanEvent;
use crate::scanners;
use crate::settings::Settings;

pub struct ScanHandle {
    cancel: Arc<AtomicBool>,
    pub events: Receiver<ScanEvent>,
}

impl ScanHandle {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

pub fn start_scan(settings: Settings) -> ScanHandle {
    let (tx, rx) = crossbeam_channel::unbounded();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_worker = cancel.clone();

    thread::spawn(move || {
        let categories = settings.scan_categories();
        let mut handles = Vec::new();

        for category in categories {
            if cancel_worker.load(Ordering::Relaxed) {
                break;
            }
            let tx_cat = tx.clone();
            let settings_cat = settings.clone();
            let cancel_cat = cancel_worker.clone();
            handles.push(thread::spawn(move || {
                if cancel_cat.load(Ordering::Relaxed) {
                    return;
                }
                scanners::run_category(category, &settings_cat, &tx_cat, &cancel_cat);
                let _ = tx_cat.send(ScanEvent::CategoryFinished { category });
            }));
        }

        for h in handles {
            let _ = h.join();
        }

        let _ = tx.send(ScanEvent::ScanFinished);
    });

    ScanHandle { cancel, events: rx }
}
