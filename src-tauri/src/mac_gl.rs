#![allow(deprecated)]
//! macOS video output. mpv's own macOS outputs ignore `wid` and open a window of their own,
//! so Flimmer drives mpv's render API instead: a CAOpenGLLayer under the webview asks mpv
//! for each frame, the way IINA does it.

use std::ffi::{c_void, CStr};
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use anyhow::{bail, Result};
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2::{define_class, msg_send, AllocAnyThread, MainThreadMarker};
use objc2_app_kit::{NSAutoresizingMaskOptions, NSView, NSWindowOrderingMode};
use objc2_quartz_core::{CALayer, CAOpenGLLayer};

use crate::mpv::ffi::{
    mpv_render_context, mpv_render_context_create, mpv_render_context_render,
    mpv_render_context_report_swap, mpv_render_context_set_update_callback,
    mpv_render_context_update, mpv_render_param as RenderParam,
    MPV_RENDER_PARAM_API_TYPE as PARAM_API_TYPE, MPV_RENDER_PARAM_FLIP_Y as PARAM_FLIP_Y,
    MPV_RENDER_PARAM_OPENGL_FBO as PARAM_OPENGL_FBO,
    MPV_RENDER_PARAM_OPENGL_INIT_PARAMS as PARAM_OPENGL_INIT_PARAMS,
    MPV_RENDER_UPDATE_FRAME as UPDATE_FRAME,
};
use crate::mpv::Mpv;

type CGLContextObj = *mut c_void;
type CGLPixelFormatObj = *mut c_void;

#[repr(C)]
struct GlInitParams {
    get_proc_address: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
    ctx: *mut c_void,
}

#[repr(C)]
struct GlFbo {
    fbo: c_int,
    w: c_int,
    h: c_int,
    internal_format: c_int,
}

const GL_FRAMEBUFFER_BINDING: u32 = 0x8CA6;
const GL_VIEWPORT: u32 = 0x0BA2;

#[link(name = "OpenGL", kind = "framework")]
extern "C" {
    fn CGLChoosePixelFormat(
        attribs: *const c_int,
        pix: *mut CGLPixelFormatObj,
        npix: *mut c_int,
    ) -> c_int;
    fn CGLCreateContext(
        pix: CGLPixelFormatObj,
        share: CGLContextObj,
        ctx: *mut CGLContextObj,
    ) -> c_int;
    fn CGLRetainPixelFormat(pix: CGLPixelFormatObj) -> CGLPixelFormatObj;
    fn CGLRetainContext(ctx: CGLContextObj) -> CGLContextObj;
    fn CGLSetCurrentContext(ctx: CGLContextObj) -> c_int;
    fn glGetIntegerv(pname: u32, data: *mut c_int);
}

extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

static PIXEL_FORMAT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static CONTEXT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static RENDER: AtomicPtr<mpv_render_context> = AtomicPtr::new(ptr::null_mut());
static LAYER: AtomicPtr<objc2::runtime::AnyObject> = AtomicPtr::new(ptr::null_mut());
/// Set by mpv's update callback; the layer asks mpv what changed on its next vsync.
static UPDATE: AtomicBool = AtomicBool::new(false);
/// Set on resize and similar: draw once even without a new frame.
static FORCE: AtomicBool = AtomicBool::new(true);

unsafe extern "C" fn get_proc_address(_ctx: *mut c_void, name: *const c_char) -> *mut c_void {
    const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;
    dlsym(RTLD_DEFAULT, name)
}

unsafe extern "C" fn on_update(_ctx: *mut c_void) {
    UPDATE.store(true, Ordering::Release);
}

define_class!(
    #[unsafe(super(CAOpenGLLayer, CALayer))]
    #[name = "FlimmerVideoLayer"]
    struct VideoLayer;

    impl VideoLayer {
        #[unsafe(method(copyCGLPixelFormatForDisplayMask:))]
        fn copy_pixel_format(&self, _mask: u32) -> CGLPixelFormatObj {
            unsafe { CGLRetainPixelFormat(PIXEL_FORMAT.load(Ordering::Acquire)) }
        }

        #[unsafe(method(copyCGLContextForPixelFormat:))]
        fn copy_context(&self, _pf: CGLPixelFormatObj) -> CGLContextObj {
            unsafe { CGLRetainContext(CONTEXT.load(Ordering::Acquire)) }
        }

        #[unsafe(method(canDrawInCGLContext:pixelFormat:forLayerTime:displayTime:))]
        fn can_draw(&self, _ctx: CGLContextObj, _pf: CGLPixelFormatObj, _t: f64, _ts: *const c_void) -> Bool {
            let render = RENDER.load(Ordering::Acquire);
            if render.is_null() {
                return Bool::NO;
            }
            let mut draw = FORCE.swap(false, Ordering::AcqRel);
            if UPDATE.swap(false, Ordering::AcqRel) {
                // SAFETY: valid render context; not called from the update callback.
                draw |= unsafe { mpv_render_context_update(render) } & UPDATE_FRAME != 0;
            }
            Bool::new(draw)
        }

        #[unsafe(method(drawInCGLContext:pixelFormat:forLayerTime:displayTime:))]
        fn draw(&self, ctx: CGLContextObj, pf: CGLPixelFormatObj, t: f64, ts: *const c_void) {
            let render = RENDER.load(Ordering::Acquire);
            if !render.is_null() {
                // SAFETY: CoreAnimation made `ctx` current and locked it for this call.
                unsafe {
                    CGLSetCurrentContext(ctx);
                    let mut fbo: c_int = 0;
                    glGetIntegerv(GL_FRAMEBUFFER_BINDING, &mut fbo);
                    let mut viewport = [0 as c_int; 4];
                    glGetIntegerv(GL_VIEWPORT, viewport.as_mut_ptr());
                    let mut target = GlFbo { fbo, w: viewport[2], h: viewport[3], internal_format: 0 };
                    let mut flip: c_int = 1;
                    let mut params = [
                        RenderParam { kind: PARAM_OPENGL_FBO, data: (&mut target as *mut GlFbo).cast() },
                        RenderParam { kind: PARAM_FLIP_Y, data: (&mut flip as *mut c_int).cast() },
                        RenderParam { kind: 0, data: ptr::null_mut() },
                    ];
                    mpv_render_context_render(render, params.as_mut_ptr());
                }
            }
            // SAFETY: the superclass flushes the context.
            unsafe {
                let _: () = msg_send![super(self), drawInCGLContext: ctx, pixelFormat: pf, forLayerTime: t, displayTime: ts];
            }
            if !render.is_null() {
                unsafe { mpv_render_context_report_swap(render) };
            }
        }
    }
);

impl VideoLayer {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

fn create_gl() -> Result<()> {
    const ACCELERATED: c_int = 73;
    const DOUBLE_BUFFER: c_int = 5;
    const COLOR_SIZE: c_int = 8;
    const ALPHA_SIZE: c_int = 11;
    const OPENGL_PROFILE: c_int = 99;
    const PROFILE_3_2_CORE: c_int = 0x3200;
    const ALLOW_OFFLINE: c_int = 96;
    const AUTO_SWITCHING: c_int = 101;
    let attribs = [
        OPENGL_PROFILE,
        PROFILE_3_2_CORE,
        ACCELERATED,
        DOUBLE_BUFFER,
        COLOR_SIZE,
        24,
        ALPHA_SIZE,
        8,
        ALLOW_OFFLINE,
        AUTO_SWITCHING,
        0,
    ];
    let mut pix: CGLPixelFormatObj = ptr::null_mut();
    let mut count = 0;
    // SAFETY: zero-terminated attribute list and out pointers.
    unsafe {
        if CGLChoosePixelFormat(attribs.as_ptr(), &mut pix, &mut count) != 0 || pix.is_null() {
            bail!("no OpenGL pixel format");
        }
        let mut ctx: CGLContextObj = ptr::null_mut();
        if CGLCreateContext(pix, ptr::null_mut(), &mut ctx) != 0 || ctx.is_null() {
            bail!("no OpenGL context");
        }
        PIXEL_FORMAT.store(pix, Ordering::Release);
        CONTEXT.store(ctx, Ordering::Release);
    }
    Ok(())
}

fn create_render_context(player: &Mpv) -> Result<()> {
    let api = c"opengl";
    let mut init = GlInitParams {
        get_proc_address,
        ctx: ptr::null_mut(),
    };
    let mut params = [
        RenderParam {
            kind: PARAM_API_TYPE,
            data: api.as_ptr() as *mut c_void,
        },
        RenderParam {
            kind: PARAM_OPENGL_INIT_PARAMS,
            data: (&mut init as *mut GlInitParams).cast(),
        },
        RenderParam {
            kind: 0,
            data: ptr::null_mut(),
        },
    ];
    let mut render: *mut mpv_render_context = ptr::null_mut();
    // SAFETY: our GL context is current on this thread; params are terminated.
    unsafe {
        CGLSetCurrentContext(CONTEXT.load(Ordering::Acquire));
        let code = mpv_render_context_create(&mut render, player.raw(), params.as_mut_ptr());
        CGLSetCurrentContext(ptr::null_mut());
        if code < 0 {
            let msg = CStr::from_ptr(crate::mpv::ffi::mpv_error_string(code)).to_string_lossy();
            bail!("mpv render context: {msg}");
        }
        mpv_render_context_set_update_callback(render, Some(on_update), ptr::null_mut());
    }
    RENDER.store(render, Ordering::Release);
    Ok(())
}

/// Puts a layer-hosting view with the video layer below the webview of `window`.
pub fn attach(window: &tauri::WebviewWindow, player: &Mpv) -> Result<()> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| anyhow::anyhow!("video view must be made on the main thread"))?;
    create_gl()?;
    create_render_context(player)?;

    // SAFETY: tauri hands out the live content view and window of this webview window.
    let content: &NSView = unsafe { &*(window.ns_view()? as *const NSView) };
    let scale: f64 = unsafe {
        let ns_window: *mut objc2::runtime::AnyObject = window.ns_window()?.cast();
        msg_send![ns_window, backingScaleFactor]
    };
    let layer = VideoLayer::new();
    unsafe {
        let _: () = msg_send![&*layer, setAsynchronous: true];
        let _: () = msg_send![&*layer, setNeedsDisplayOnBoundsChange: true];
        let _: () = msg_send![&*layer, setContentsScale: scale];
        let _: () = msg_send![&*layer, setOpaque: false];
    }
    LAYER.store(Retained::as_ptr(&layer) as *mut _, Ordering::Release);
    let view = NSView::initWithFrame(mtm.alloc::<NSView>(), content.bounds());
    view.setLayer(Some(&layer));
    view.setWantsLayer(true);
    view.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    content.addSubview_positioned_relativeTo(&view, NSWindowOrderingMode::Below, None);
    // The window owns the view from here on; the layer lives as long as the view.
    std::mem::forget(view);
    Ok(())
}

/// Follows the window to a screen with another backing scale. Main thread only.
pub fn set_scale(scale: f64) {
    let layer = LAYER.load(Ordering::Acquire);
    if !layer.is_null() {
        // SAFETY: the layer lives as long as the window's view.
        unsafe {
            let _: () = msg_send![layer, setContentsScale: scale];
        }
        FORCE.store(true, Ordering::Release);
    }
}
