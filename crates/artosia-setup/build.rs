//! Puts Artosia's mark and the manifest on the setup file. The manifest asks for common
//! controls version 6, without which `TaskDialogIndirect` is not found when the file starts.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/brand/artosia.ico");
    println!("cargo:rerun-if-changed=setup.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    // The linker stamps the build time into the file unless told not to; a setup that is kept the
    // same file across releases should also be the same bytes when built again.
    println!("cargo:rustc-link-arg-bins=/Brepro");
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("../../assets/brand/artosia.ico");
    resource.set_manifest_file("setup.manifest");
    resource.set("ProductName", "Artosia setup");
    resource.set("FileDescription", "Artosia setup");
    if let Err(error) = resource.compile() {
        panic!("the setup file needs its manifest, and the resource compiler failed: {error}");
    }
}
