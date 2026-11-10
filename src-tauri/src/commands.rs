//! The page's way to mpv. All commands are async so they run off the main thread: mpv's
//! macOS video output dispatches to the main thread synchronously and would deadlock.

use std::sync::Arc;

use tauri::State;

use crate::mpv::Mpv;
use crate::Pending;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

#[tauri::command]
pub async fn mpv_command(player: State<'_, Arc<Mpv>>, args: Vec<String>) -> Result<(), String> {
    player.command(&args).map_err(err)
}

#[tauri::command]
pub async fn mpv_set(player: State<'_, Arc<Mpv>>, name: String, value: String) -> Result<(), String> {
    player.set(&name, &value).map_err(err)
}

#[tauri::command]
pub async fn mpv_get(player: State<'_, Arc<Mpv>>, name: String) -> Result<Option<String>, String> {
    Ok(player.get(&name))
}

/// Plays the first file now and queues the rest behind it. Saves the position of what
/// played before, so it resumes there next time.
#[tauri::command]
pub async fn open_files(player: State<'_, Arc<Mpv>>, files: Vec<String>, append: bool) -> Result<(), String> {
    if !append {
        let _ = player.command(&["write-watch-later-config".into()]);
    }
    for (i, file) in files.iter().enumerate() {
        let mode = if append || i > 0 { "append-play" } else { "replace" };
        player.command(&["loadfile".into(), file.clone(), mode.into()]).map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn save_position(player: State<'_, Arc<Mpv>>) -> Result<(), String> {
    player.command(&["write-watch-later-config".into()]).map_err(err)
}

#[tauri::command]
pub fn pending_files(pending: State<'_, Pending>) -> Vec<String> {
    std::mem::take(&mut *pending.0.lock().unwrap())
}

/// Streams filmstrip stills for `path` through `channel` as they are rendered.
#[tauri::command]
pub async fn filmstrip(path: String, count: usize, width: u32, channel: tauri::ipc::Channel<crate::thumbs::Still>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::thumbs::filmstrip(&path, count.clamp(1, 40), width.clamp(32, 640), |still| {
            let _ = channel.send(still);
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(err)
}

#[tauri::command]
pub async fn edge_colors(player: State<'_, Arc<Mpv>>) -> Result<[[u8; 3]; 4], String> {
    player.edge_colors().map_err(err)
}

#[derive(serde::Serialize)]
pub struct Recent {
    path: String,
    start: f64,
    saved: u64,
}

/// What mpv's watch-later files say was played last: path, position, when. Newest first.
#[tauri::command]
pub async fn recent_files(app: tauri::AppHandle) -> Result<Vec<Recent>, String> {
    use tauri::Manager;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("watch_later");
    let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(Vec::new()) };
    let mut out: Vec<Recent> = entries
        .flatten()
        .filter_map(|e| {
            let saved = e.metadata().ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
            parse_watch_later(&std::fs::read_to_string(e.path()).ok()?).map(|(path, start)| Recent { path, start, saved })
        })
        .collect();
    out.sort_by(|a, b| b.saved.cmp(&a.saved));
    out.truncate(24);
    Ok(out)
}

/// `# <path>` then `key=value` lines; files without a path line are skipped.
pub fn parse_watch_later(text: &str) -> Option<(String, f64)> {
    let mut lines = text.lines();
    let path = lines.next()?.strip_prefix("# ")?.to_string();
    let start = lines.find_map(|l| l.strip_prefix("start=")).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    Some((path, start))
}

/// Shows or hides the traffic lights together with the rest of the controls.
#[tauri::command]
pub fn window_buttons(window: tauri::WebviewWindow, visible: bool) {
    #[cfg(target_os = "macos")]
    {
        let Ok(ns_window) = window.ns_window() else { return };
        let ns_window = ns_window as usize;
        let _ = window.run_on_main_thread(move || {
            use objc2::msg_send;
            use objc2::runtime::AnyObject;
            let w = ns_window as *mut AnyObject;
            for kind in 0usize..3 {
                // SAFETY: the live NSWindow; standardWindowButton: 0..2 are close, minimize, zoom.
                unsafe {
                    let button: *mut AnyObject = msg_send![w, standardWindowButton: kind];
                    if !button.is_null() {
                        let _: () = msg_send![button, setHidden: !visible];
                    }
                }
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (window, visible);
}

#[cfg(test)]
mod tests {
    use super::parse_watch_later;

    #[test]
    fn watch_later_gives_path_and_start() {
        let text = "# /Users/me/Filme/Kochelsee.mkv\nstart=2472.120000\nsid=1\n";
        assert_eq!(parse_watch_later(text), Some(("/Users/me/Filme/Kochelsee.mkv".into(), 2472.12)));
        assert_eq!(parse_watch_later("start=1\n"), None);
    }
}
