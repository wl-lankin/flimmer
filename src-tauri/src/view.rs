//! Windows video output: mpv makes its own child window inside the main window, below the
//! WebView2 controller, so the page decides what of the video shows.

#[cfg(target_os = "windows")]
pub fn wid(window: &tauri::WebviewWindow) -> anyhow::Result<i64> {
    Ok(window.hwnd()?.0 as i64)
}
