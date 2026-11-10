//! Filmstrip stills from a second, invisible mpv that renders in software, so it needs no
//! window or GPU context and works the same on every platform.

use std::ffi::c_void;
use std::os::raw::c_int;
use std::ptr;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use base64::Engine;
use serde::Serialize;

use crate::mpv::ffi::*;
use crate::mpv::Mpv;

#[derive(Clone, Serialize)]
pub struct Still {
    pub index: usize,
    pub time: f64,
    pub url: String,
}

struct Renderer {
    mpv: std::sync::Arc<Mpv>,
    render: *mut mpv_render_context,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        // SAFETY: the render context goes first, then the handle; neither is used again.
        unsafe {
            if !self.render.is_null() {
                mpv_render_context_free(self.render);
            }
            self.mpv.destroy();
        }
    }
}

fn wait_for(mpv: &Mpv, event: i32, limit: Duration) -> Result<()> {
    let start = Instant::now();
    while start.elapsed() < limit {
        match mpv.wait_event(0.25) {
            e if e == event => return Ok(()),
            MPV_EVENT_END_FILE | MPV_EVENT_SHUTDOWN => bail!("thumbnailer: file ended"),
            _ => {}
        }
    }
    bail!("thumbnailer: timed out")
}

/// Renders `count` stills, evenly spread, `width` pixels wide, and hands each to `each`
/// as soon as it exists.
pub fn filmstrip(path: &str, count: usize, width: u32, mut each: impl FnMut(Still)) -> Result<()> {
    let options: Vec<(&str, String)> = [
        ("vo", "libmpv"),
        ("config", "no"),
        ("hwdec", "no"),
        ("aid", "no"),
        ("sid", "no"),
        ("pause", "yes"),
        ("keep-open", "always"),
        ("idle", "yes"),
        ("ytdl", "no"),
        ("load-scripts", "no"),
        ("osc", "no"),
        ("hr-seek", "no"),
        ("vd-lavc-fast", "yes"),
        ("vd-lavc-skiploopfilter", "all"),
        ("vd-lavc-threads", "2"),
        ("demuxer-max-bytes", "16MiB"),
        ("resume-playback", "no"),
        ("tone-mapping", "auto"),
    ]
    .into_iter()
    .map(|(k, v)| (k, v.to_string()))
    .collect();
    let mpv = Mpv::new(&options)?;
    let mut r = Renderer { mpv, render: ptr::null_mut() };
    let api = c"sw";
    let mut params = [
        mpv_render_param { kind: MPV_RENDER_PARAM_API_TYPE, data: api.as_ptr() as *mut c_void },
        mpv_render_param { kind: 0, data: ptr::null_mut() },
    ];
    // SAFETY: valid handle; params are terminated.
    if unsafe { mpv_render_context_create(&mut r.render, r.mpv.raw(), params.as_mut_ptr()) } < 0 {
        bail!("thumbnailer: no software render context");
    }
    r.mpv.command(&["loadfile".into(), path.into()])?;
    wait_for(&r.mpv, MPV_EVENT_FILE_LOADED, Duration::from_secs(15))?;

    let duration: f64 = r.mpv.get("duration").and_then(|d| d.parse().ok()).unwrap_or(0.0);
    let dw: f64 = r.mpv.get("video-params/dw").and_then(|d| d.parse().ok()).unwrap_or(0.0);
    let dh: f64 = r.mpv.get("video-params/dh").and_then(|d| d.parse().ok()).unwrap_or(0.0);
    if duration <= 0.0 {
        bail!("thumbnailer: no duration");
    }
    let aspect = if dw > 0.0 && dh > 0.0 { dw / dh } else { 16.0 / 9.0 };
    let w = width.max(16) as c_int;
    let h = ((width as f64 / aspect).round() as c_int).max(2) & !1;
    let stride = ((w as usize * 4) + 63) & !63;
    let mut buffer = vec![0u8; stride * h as usize];

    for index in 0..count {
        let time = duration * (index as f64 + 0.5) / count as f64;
        r.mpv.command(&["seek".into(), format!("{time:.3}"), "absolute+keyframes".into()])?;
        if wait_for(&r.mpv, MPV_EVENT_PLAYBACK_RESTART, Duration::from_secs(8)).is_err() {
            continue;
        }
        let deadline = Instant::now() + Duration::from_millis(1500);
        let mut drawn = false;
        while Instant::now() < deadline {
            // SAFETY: valid render context, used from this thread only.
            if unsafe { mpv_render_context_update(r.render) } & MPV_RENDER_UPDATE_FRAME != 0 {
                let mut size = [w, h];
                let format = c"rgb0";
                let mut stride_v = stride;
                let mut params = [
                    mpv_render_param { kind: MPV_RENDER_PARAM_SW_SIZE, data: size.as_mut_ptr().cast() },
                    mpv_render_param { kind: MPV_RENDER_PARAM_SW_FORMAT, data: format.as_ptr() as *mut c_void },
                    mpv_render_param { kind: MPV_RENDER_PARAM_SW_STRIDE, data: (&mut stride_v as *mut usize).cast() },
                    mpv_render_param { kind: MPV_RENDER_PARAM_SW_POINTER, data: buffer.as_mut_ptr().cast() },
                    mpv_render_param { kind: 0, data: ptr::null_mut() },
                ];
                // SAFETY: the buffer holds stride * h bytes as declared.
                drawn = unsafe { mpv_render_context_render(r.render, params.as_mut_ptr()) } >= 0;
                break;
            }
            r.mpv.wait_event(0.02);
        }
        if !drawn {
            continue;
        }
        let mut rgb = Vec::with_capacity(w as usize * h as usize * 3);
        for row in buffer.chunks(stride).take(h as usize) {
            for px in row[..w as usize * 4].chunks_exact(4) {
                rgb.extend_from_slice(&px[..3]);
            }
        }
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 72).encode(&rgb, w as u32, h as u32, image::ExtendedColorType::Rgb8)?;
        let url = format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(&jpeg));
        each(Still { index, time, url });
    }
    Ok(())
}
