mod commands;
#[cfg(target_os = "macos")]
mod mac_gl;
mod mpv;
#[cfg(target_os = "windows")]
mod view;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{Emitter, Manager};

use mpv::Mpv;

pub const MAIN_LABEL: &str = "main";

/// Files handed to Flimmer before the page could take them (command line, Finder "Open
/// With", a second instance). The page drains the list once it is ready.
#[derive(Default)]
pub struct Pending(pub Mutex<Vec<String>>);

/// Properties the page follows. Their order is their reply id, so append only.
const OBSERVED: &[&str] = &[
    "time-pos",
    "duration",
    "pause",
    "idle-active",
    "eof-reached",
    "volume",
    "mute",
    "speed",
    "media-title",
    "path",
    "track-list",
    "chapter-list",
    "chapter",
    "video-params",
    "video-format",
    "audio-codec-name",
    "hwdec-current",
    "demuxer-cache-time",
    "paused-for-cache",
    "sub-delay",
    "audio-delay",
    "ab-loop-a",
    "ab-loop-b",
    "estimated-vf-fps",
    "playlist-pos",
    "playlist-count",
];

fn watch_later_dir(app: &tauri::App) -> Option<PathBuf> {
    let dir = app.path().app_data_dir().ok()?.join("watch_later");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn setup(app: &mut tauri::App) -> anyhow::Result<()> {
    let window = app.get_webview_window(MAIN_LABEL).ok_or_else(|| anyhow::anyhow!("no main window"))?;
    let mut options: Vec<(&str, String)> = vec![
        ("hwdec", "auto-safe".into()),
        ("config", "no".into()),
        ("idle", "yes".into()),
        ("keep-open", "yes".into()),
        ("force-window", "no".into()),
        ("input-default-bindings", "no".into()),
        ("input-vo-keyboard", "no".into()),
        ("input-cursor", "no".into()),
        ("cursor-autohide", "no".into()),
        ("osc", "no".into()),
        ("osd-level", "0".into()),
        ("osd-bar", "no".into()),
        ("sub-auto", "fuzzy".into()),
        ("audio-file-auto", "fuzzy".into()),
        ("ytdl", "yes".into()),
        ("target-colorspace-hint", "yes".into()),
        ("resume-playback", "yes".into()),
        ("save-position-on-quit", "yes".into()),
        ("watch-later-options", "start,vid,aid,sid,speed,sub-delay,audio-delay,volume,mute".into()),
    ];
    #[cfg(target_os = "macos")]
    options.push(("vo", "libmpv".into()));
    #[cfg(target_os = "windows")]
    {
        options.push(("wid", view::wid(&window)?.to_string()));
        options.push(("vo", "gpu-next".into()));
    }
    if let Some(dir) = watch_later_dir(app) {
        options.push(("watch-later-dir", dir.to_string_lossy().into_owned()));
    }
    let player = Mpv::new(&options)?;
    #[cfg(target_os = "macos")]
    mac_gl::attach(&window, &player)?;
    player.observe(OBSERVED);
    let handle = app.handle().clone();
    player.spawn_events(move |event| {
        let _ = handle.emit_to(MAIN_LABEL, "mpv", event);
    });
    let (major, minor) = Mpv::api_version();
    eprintln!("libmpv client API {major}.{minor}");
    app.manage(player);

    let args: Vec<String> = std::env::args().skip(1).filter(|a| !a.starts_with('-')).collect();
    app.state::<Pending>().0.lock().unwrap().extend(args);
    Ok(())
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let files: Vec<String> = argv.into_iter().skip(1).filter(|a| !a.starts_with('-')).collect();
            if !files.is_empty() {
                let _ = app.emit_to(MAIN_LABEL, "open-files", files);
            }
            if let Some(w) = app.get_webview_window(MAIN_LABEL) {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(Pending::default())
        .setup(|app| {
            setup(app).map_err(|e| {
                eprintln!("setup failed: {e:#}");
                e.into()
            })
        })
        .invoke_handler(tauri::generate_handler![
            commands::mpv_command,
            commands::mpv_set,
            commands::mpv_get,
            commands::open_files,
            commands::pending_files,
            commands::save_position,
        ])
        .build(tauri::generate_context!())
        .expect("building the app");

    app.run(|handle, event| match event {
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Opened { urls } => {
            let files: Vec<String> = urls
                .into_iter()
                .filter_map(|u| u.to_file_path().ok().map(|p| p.to_string_lossy().into_owned()).or(Some(u.to_string())))
                .collect();
            handle.state::<Pending>().0.lock().unwrap().extend(files.clone());
            let _ = handle.emit_to(MAIN_LABEL, "open-files", files);
        }
        tauri::RunEvent::ExitRequested { .. } => {
            if let Some(player) = handle.try_state::<Arc<Mpv>>() {
                let _ = player.command(&["write-watch-later-config".into()]);
                player.quit();
            }
        }
        _ => {}
    });
}
