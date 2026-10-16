fn main() {
    println!("cargo:rerun-if-changed=assets/clicker-loop-icon.png");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let source = image::open("assets/clicker-loop-icon.png").expect("Read Clicker Loop icon");
    let output =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("clicker-loop.ico");
    source
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3)
        .save_with_format(&output, image::ImageFormat::Ico)
        .expect("Create Windows icon");
    winresource::WindowsResource::new()
        .set_icon(output.to_str().expect("Icon path"))
        .set("FileDescription", "Clicker Loop - Autoclicker")
        .set("ProductName", "Clicker Loop - Autoclicker")
        .compile()
        .expect("Embed Windows icon resource");
}
