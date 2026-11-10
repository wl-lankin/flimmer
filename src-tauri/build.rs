fn main() {
    // libmpv. MPV_LIB_DIR wins (CI and release builds point it at mpv-build/out/<target>); on a
    // Mac the Homebrew build is the development default. Windows binds libmpv-2.dll by name
    // (raw-dylib) and needs no import library.
    let macos = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos");
    if let Ok(dir) = std::env::var("MPV_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
        if macos && std::env::var("PROFILE").as_deref() != Ok("release") {
            // Our build names itself @rpath/libmpv.2.dylib; let dev runs find it where it lies.
            println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
        }
    } else if macos {
        println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
    }
    if macos {
        // whisper.cpp's Metal code asks for the OS version at run time (@available), which
        // lives in clang's runtime; Rust does not link it on its own.
        if let Ok(out) = std::process::Command::new("clang")
            .arg("--print-runtime-dir")
            .output()
        {
            let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if std::path::Path::new(&dir)
                .join("libclang_rt.osx.a")
                .exists()
            {
                println!("cargo:rustc-link-search=native={dir}");
                println!("cargo:rustc-link-lib=static=clang_rt.osx");
            }
        }
        // The bundle carries libmpv in Contents/Frameworks.
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
    }
    println!("cargo:rerun-if-env-changed=MPV_LIB_DIR");
    tauri_build::build()
}
