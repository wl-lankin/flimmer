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
