//! Embeds the icon and version info in the Windows executable.

fn main() {
    println!("cargo:rerun-if-changed=../app/wix/DevCleaner.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../app/wix/DevCleaner.ico");
    res.set("ProductName", "DevCleaner");
    res.set("FileDescription", "DevCleaner");
    res.set("LegalCopyright", "MIT License");
    if let Err(e) = res.compile() {
        println!("cargo:warning=could not embed the Windows icon: {e}");
    }
}
