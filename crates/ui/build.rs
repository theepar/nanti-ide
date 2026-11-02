// Embeds the app icon and version info into nanti-ui.exe (Windows only).
fn main() {
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/nanti.ico");
    #[cfg(windows)]
    embed_resource::compile("assets/app.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("cannot embed the Windows resources");
}
