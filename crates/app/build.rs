//! Embeds the icon and version info in the Windows executable.

fn main() {
    println!("cargo:rerun-if-changed=wix/DevCleaner.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("wix/DevCleaner.ico");
    res.set("ProductName", "DevCleaner");
    res.set("FileDescription", "DevCleaner");
    res.set("LegalCopyright", "MIT License");
    // A missing resource compiler should not break the build; the app just
    // falls back to the default icon.
    if let Err(e) = res.compile() {
        println!("cargo:warning=could not embed the Windows icon: {e}");
    }
}
