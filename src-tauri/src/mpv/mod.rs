//! A thin, thread-safe wrapper over libmpv's client API.
//!
//! Every property the page needs is observed as a string: mpv prints scalars plainly
//! ("41.250000", "yes") and nodes such as `track-list` as JSON, so one event shape carries
//! everything and the page parses what it needs.

pub mod ffi;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Result};
use serde::Serialize;

use ffi::*;

pub struct Mpv {
    ctx: *mut mpv_handle,
}

// SAFETY: the libmpv client API is thread-safe for a single handle (client.h, "Thread safety").
unsafe impl Send for Mpv {}
unsafe impl Sync for Mpv {}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Event {
    Prop {
        name: String,
        value: Option<String>,
    },
    StartFile,
    FileLoaded,
    EndFile {
        reason: &'static str,
        error: Option<String>,
    },
    VideoReconfig,
    Seek,
    PlaybackRestart,
    Shutdown,
}

/// Commands that would let the page run programs or scripts through mpv.
const BLOCKED_COMMANDS: &[&str] = &[
    "run",
    "subprocess",
    "load-script",
    "load-config-file",
    "load-input-conf",
];

fn cstring(s: &str) -> Result<CString> {
    CString::new(s).map_err(|_| anyhow!("NUL byte in mpv argument"))
}

fn check(code: i32, what: &str) -> Result<()> {
    if code >= 0 {
        return Ok(());
    }
    // SAFETY: mpv_error_string returns a static string for every code.
    let msg = unsafe { CStr::from_ptr(mpv_error_string(code)) }.to_string_lossy();
    bail!("{what}: {msg}")
}

/// Takes ownership of a string mpv allocated.
unsafe fn take_string(p: *mut c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    let s = CStr::from_ptr(p).to_string_lossy().into_owned();
    mpv_free(p.cast());
    Some(s)
}

/// Reads screenshot-raw's map (w, h, stride, format, data) and averages the edge bands.
unsafe fn frame_edges(node: &mpv_node) -> Result<[[u8; 3]; 4]> {
    if node.format != MPV_FORMAT_NODE_MAP {
        bail!("screenshot-raw gave no map");
    }
    let list = &*node.u.list;
    let (mut w, mut h, mut stride, mut format, mut data) =
        (0i64, 0i64, 0i64, String::new(), (ptr::null::<u8>(), 0usize));
    for i in 0..list.num as usize {
        let key = CStr::from_ptr(*list.keys.add(i)).to_string_lossy();
        let v = &*list.values.add(i);
        match (key.as_ref(), v.format) {
            ("w", MPV_FORMAT_INT64) => w = v.u.int64,
            ("h", MPV_FORMAT_INT64) => h = v.u.int64,
            ("stride", MPV_FORMAT_INT64) => stride = v.u.int64,
            ("format", MPV_FORMAT_STRING) => {
                format = CStr::from_ptr(v.u.string).to_string_lossy().into_owned()
            }
            ("data", MPV_FORMAT_BYTE_ARRAY) => {
                let ba = &*v.u.ba;
                data = (ba.data as *const u8, ba.size);
            }
            _ => {}
        }
    }
    if w <= 0 || h <= 0 || data.0.is_null() || (stride * h) as usize > data.1 {
        bail!("screenshot-raw gave an unexpected frame");
    }
    let bytes = std::slice::from_raw_parts(data.0, data.1);
    // bgr0 and bgra keep red at offset 2; rgba and rgb0 at 0.
    let (r_at, b_at) = if format.starts_with("bgr") {
        (2, 0)
    } else {
        (0, 2)
    };
    Ok(edge_averages(
        bytes,
        w as usize,
        h as usize,
        stride as usize,
        r_at,
        b_at,
    ))
}

/// Averages four bands (left, right, top, bottom; each 18 % deep) of a 4-byte-per-pixel frame,
/// sampling a sparse grid so a 4K frame costs a few thousand reads.
pub fn edge_averages(
    bytes: &[u8],
    w: usize,
    h: usize,
    stride: usize,
    r_at: usize,
    b_at: usize,
) -> [[u8; 3]; 4] {
    let band_w = (w * 18 / 100).max(1);
    let band_h = (h * 18 / 100).max(1);
    let step = (w.max(h) / 96).max(1);
    let mut sums = [[0u64; 4]; 4];
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let i = y * stride + x * 4;
            let px = [
                bytes[i + r_at] as u64,
                bytes[i + 1] as u64,
                bytes[i + b_at] as u64,
            ];
            let bands = [x < band_w, x >= w - band_w, y < band_h, y >= h - band_h];
            for (band, inside) in bands.iter().enumerate() {
                if *inside {
                    for c in 0..3 {
                        sums[band][c] += px[c];
                    }
                    sums[band][3] += 1;
                }
            }
            x += step;
        }
        y += step;
    }
    sums.map(|s| {
        let n = s[3].max(1);
        [(s[0] / n) as u8, (s[1] / n) as u8, (s[2] / n) as u8]
    })
}

impl Mpv {
    /// Creates a handle with `options` applied before initialization (`wid`, `vo`, ...).
    pub fn new(options: &[(&str, String)]) -> Result<Arc<Self>> {
        // SAFETY: plain constructor; null means out of memory.
        let ctx = unsafe { mpv_create() };
        if ctx.is_null() {
            bail!("mpv_create failed");
        }
        let mpv = Mpv { ctx };
        for (name, value) in options {
            let (n, v) = (cstring(name)?, cstring(value)?);
            // SAFETY: valid handle and NUL-terminated strings.
            let code = unsafe { mpv_set_option_string(ctx, n.as_ptr(), v.as_ptr()) };
            if let Err(err) = check(code, &format!("option {name}={value}")) {
                eprintln!("{err}");
            }
        }
        let level = cstring("warn")?;
        // SAFETY: valid handle.
        unsafe {
            mpv_request_log_messages(ctx, level.as_ptr());
            check(mpv_initialize(ctx), "mpv_initialize")?;
        }
        Ok(Arc::new(mpv))
    }

    pub fn command(&self, args: &[String]) -> Result<()> {
        let Some(first) = args.first() else {
            bail!("empty mpv command")
        };
        if BLOCKED_COMMANDS.contains(&first.as_str()) {
            bail!("mpv command '{first}' is not allowed");
        }
        let owned = args
            .iter()
            .map(|a| cstring(a))
            .collect::<Result<Vec<_>>>()?;
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(ptr::null());
        // SAFETY: NULL-terminated array of C strings that outlive the call.
        check(
            unsafe { mpv_command(self.ctx, ptrs.as_mut_ptr()) },
            &format!("command {first}"),
        )
    }

    /// Grabs the current video frame (without subtitles) and returns the average colour
    /// of its four edge bands: left, right, top, bottom.
    pub fn edge_colors(&self) -> Result<[[u8; 3]; 4]> {
        let args = [cstring("screenshot-raw")?, cstring("video")?];
        let mut ptrs: Vec<*const c_char> = args.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(ptr::null());
        // SAFETY: zeroed node is a valid MPV_FORMAT_NONE node for mpv to fill.
        let mut node: mpv_node = unsafe { std::mem::zeroed() };
        check(
            unsafe { mpv_command_ret(self.ctx, ptrs.as_mut_ptr(), &mut node) },
            "screenshot-raw",
        )?;
        // SAFETY: on success mpv filled a node map; it is freed below in every case.
        let result = unsafe { frame_edges(&node) };
        unsafe { mpv_free_node_contents(&mut node) };
        result
    }

    pub fn set(&self, name: &str, value: &str) -> Result<()> {
        let (n, v) = (cstring(name)?, cstring(value)?);
        // SAFETY: valid handle and strings.
        check(
            unsafe { mpv_set_property_string(self.ctx, n.as_ptr(), v.as_ptr()) },
            &format!("set {name}"),
        )
    }

    pub fn get(&self, name: &str) -> Option<String> {
        let n = cstring(name).ok()?;
        // SAFETY: valid handle; the result is ours to free.
        unsafe { take_string(mpv_get_property_string(self.ctx, n.as_ptr())) }
    }

    pub fn observe(&self, names: &[&str]) {
        for (i, name) in names.iter().enumerate() {
            let Ok(n) = cstring(name) else { continue };
            // SAFETY: valid handle; mpv copies the name.
            let code = unsafe {
                mpv_observe_property(self.ctx, i as u64 + 1, n.as_ptr(), MPV_FORMAT_STRING)
            };
            if let Err(err) = check(code, &format!("observe {name}")) {
                eprintln!("{err}");
            }
        }
    }

    /// Runs the event loop on its own thread until mpv shuts down. Frequent properties are
    /// thinned out so the page gets at most one update per `THROTTLE`.
    pub fn spawn_events(self: &Arc<Self>, mut sink: impl FnMut(Event) + Send + 'static) {
        const THROTTLE: Duration = Duration::from_millis(100);
        const FREQUENT: &[&str] = &[
            "time-pos",
            "demuxer-cache-time",
            "estimated-vf-fps",
            "avsync",
        ];
        let mpv = Arc::clone(self);
        std::thread::Builder::new()
            .name("mpv-events".into())
            .spawn(move || {
                let mut last: Vec<(String, Instant)> = Vec::new();
                loop {
                    // SAFETY: valid handle; the event stays valid until the next wait.
                    let ev = unsafe { &*mpv_wait_event(mpv.ctx, -1.0) };
                    let out = match ev.event_id {
                        MPV_EVENT_SHUTDOWN => {
                            sink(Event::Shutdown);
                            break;
                        }
                        MPV_EVENT_PROPERTY_CHANGE => {
                            // SAFETY: data is an mpv_event_property for this event.
                            let p = unsafe { &*(ev.data as *const mpv_event_property) };
                            let name = unsafe { CStr::from_ptr(p.name) }
                                .to_string_lossy()
                                .into_owned();
                            let value = if p.format == MPV_FORMAT_STRING && !p.data.is_null() {
                                // SAFETY: for MPV_FORMAT_STRING, data points at a char*.
                                let s = unsafe { *(p.data as *const *const c_char) };
                                (!s.is_null()).then(|| {
                                    unsafe { CStr::from_ptr(s) }.to_string_lossy().into_owned()
                                })
                            } else {
                                None
                            };
                            if FREQUENT.contains(&name.as_str()) {
                                let now = Instant::now();
                                match last.iter_mut().find(|(n, _)| *n == name) {
                                    Some((_, t)) if now.duration_since(*t) < THROTTLE => continue,
                                    Some((_, t)) => *t = now,
                                    None => last.push((name.clone(), now)),
                                }
                            }
                            Event::Prop { name, value }
                        }
                        MPV_EVENT_START_FILE => Event::StartFile,
                        MPV_EVENT_FILE_LOADED => Event::FileLoaded,
                        MPV_EVENT_END_FILE => {
                            // SAFETY: data is an mpv_event_end_file for this event.
                            let e = unsafe { &*(ev.data as *const mpv_event_end_file) };
                            let reason = match e.reason {
                                0 => "eof",
                                2 => "stop",
                                3 => "quit",
                                4 => "error",
                                5 => "redirect",
                                _ => "other",
                            };
                            let error = (e.reason == MPV_END_FILE_REASON_ERROR).then(|| {
                                unsafe { CStr::from_ptr(mpv_error_string(e.error)) }
                                    .to_string_lossy()
                                    .into_owned()
                            });
                            Event::EndFile { reason, error }
                        }
                        MPV_EVENT_VIDEO_RECONFIG => Event::VideoReconfig,
                        MPV_EVENT_SEEK => Event::Seek,
                        MPV_EVENT_PLAYBACK_RESTART => Event::PlaybackRestart,
                        MPV_EVENT_LOG_MESSAGE => {
                            // SAFETY: data is an mpv_event_log_message for this event.
                            let m = unsafe { &*(ev.data as *const mpv_event_log_message) };
                            let (prefix, text) = unsafe {
                                (
                                    CStr::from_ptr(m.prefix).to_string_lossy(),
                                    CStr::from_ptr(m.text).to_string_lossy(),
                                )
                            };
                            eprint!("[mpv/{prefix}] {text}");
                            continue;
                        }
                        _ => continue,
                    };
                    sink(out);
                }
            })
            .expect("spawning the mpv event thread");
    }

    /// Asks mpv to shut down; the event thread ends on the resulting shutdown event.
    pub fn quit(&self) {
        let _ = self.command(&["quit".into()]);
        // SAFETY: valid handle.
        unsafe { mpv_wakeup(self.ctx) };
    }

    /// Waits up to `timeout` seconds for the next event and returns its id (0 on timeout).
    /// Only for handles without an event thread, such as the thumbnailer's.
    pub fn wait_event(&self, timeout: f64) -> i32 {
        // SAFETY: valid handle.
        unsafe { (*mpv_wait_event(self.ctx, timeout)).event_id }
    }

    /// Destroys the handle. Any render context must be freed before.
    ///
    /// # Safety
    /// Nothing may use this handle afterwards.
    pub unsafe fn destroy(&self) {
        mpv_terminate_destroy(self.ctx);
    }

    pub fn raw(&self) -> *mut mpv_handle {
        self.ctx
    }

    pub fn api_version() -> (u32, u32) {
        // SAFETY: pure function.
        let v = unsafe { mpv_client_api_version() } as u32;
        (v >> 16, v & 0xffff)
    }
}

#[cfg(test)]
mod tests {
    use super::edge_averages;

    #[test]
    fn edge_bands_average_their_own_side() {
        // 100x50 bgr0 frame: left half red, right half blue.
        let (w, h) = (100, 50);
        let mut frame = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                if x < w / 2 {
                    frame[i + 2] = 255
                } else {
                    frame[i] = 255
                }
            }
        }
        let [left, right, top, _] = edge_averages(&frame, w, h, w * 4, 2, 0);
        assert_eq!(left, [255, 0, 0]);
        assert_eq!(right, [0, 0, 255]);
        assert!(top[0] > 100 && top[2] > 100);
    }
}
