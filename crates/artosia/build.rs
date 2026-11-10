//! Puts Artosia's mark on the program file, so Explorer and a desktop shortcut show it.
//! The window and the taskbar get theirs at run time (`rune::set_app_icon` in `main`).

fn main() {
    println!("cargo:rerun-if-changed=../../assets/brand/artosia.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("../../assets/brand/artosia.ico");
    // A machine without the Windows SDK's resource compiler still builds, with a plain icon.
    if let Err(error) = resource.compile() {
        println!("cargo:warning=the program file has no icon: {error}");
    }
}
