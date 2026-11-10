fn main() {
    // libmpv: MPV_LIB_DIR wins; on a Mac the Homebrew build is the development default.
    if let Ok(dir) = std::env::var("MPV_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
    } else if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
    }
    println!("cargo:rerun-if-env-changed=MPV_LIB_DIR");
    tauri_build::build()
}
