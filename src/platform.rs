#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlassBackdrop {
    Acrylic,
    Mica,
    None,
}

#[cfg(windows)]
pub const MAIN_VIEWPORT_TITLE: &str = "bd-neon";

/// True when `--glass-debug` is on the command line or
/// `BD_NEON_GLASS_DEBUG=1` is set. Enables verbose DWM logging and a
/// tinted debug overlay in `window_chrome::shell`.
#[must_use]
pub fn glass_debug_enabled() -> bool {
    std::env::args().any(|arg| arg == "--glass-debug")
        || std::env::var("BD_NEON_GLASS_DEBUG").as_deref() == Ok("1")
}

#[cfg(windows)]
static LAST_LOGGED_BACKDROP: std::sync::atomic::AtomicI8 = std::sync::atomic::AtomicI8::new(-1);

#[cfg(windows)]
static BUILD_LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(windows)]
pub mod win_cast {
    #[allow(clippy::as_conversions)]
    pub const fn raw_to_mut_c_void(raw: isize) -> *mut core::ffi::c_void {
        raw as *mut core::ffi::c_void
    }

    pub fn size_of_u32<T>() -> u32 {
        u32::try_from(std::mem::size_of::<T>()).unwrap_or(u32::MAX)
    }

    #[allow(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub const fn long_ptr_to_u32(value: isize) -> u32 {
        value as u32
    }

    #[allow(clippy::as_conversions, clippy::cast_possible_wrap)]
    pub const fn u32_to_long_ptr(value: u32) -> isize {
        value as isize
    }
}

/// Human-readable Windows version for `[glass]` diagnostics.
/// Uses `GetVersionExW` (may report compat-mode values without a versioned
/// manifest; still identifies the build family for backdrop debugging).
#[cfg(windows)]
pub fn log_windows_build() {
    use windows::Win32::System::SystemInformation::{GetVersion, GetVersionExW, OSVERSIONINFOW};
    if BUILD_LOGGED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    unsafe {
        let mut info = OSVERSIONINFOW {
            dwOSVersionInfoSize: win_cast::size_of_u32::<OSVERSIONINFOW>(),
            ..Default::default()
        };
        let result = GetVersionExW(&raw mut info);
        let raw = GetVersion();
        match result {
            Ok(()) => eprintln!(
                "[glass] windows build: {}.{}.{} raw=0x{raw:08X} debug={}",
                info.dwMajorVersion,
                info.dwMinorVersion,
                info.dwBuildNumber,
                glass_debug_enabled()
            ),
            Err(err) => eprintln!(
                "[glass] GetVersionExW failed hr=0x{:08X} raw=0x{raw:08X}",
                err.code().0
            ),
        }
    }
}

#[cfg(windows)]
fn hwnd_from_frame(frame: &eframe::Frame) -> Option<windows::Win32::Foundation::HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;

    let handle = frame.window_handle().ok()?;
    let RawWindowHandle::Win32(win) = handle.as_raw() else {
        return None;
    };
    Some(HWND(win_cast::raw_to_mut_c_void(win.hwnd.get())))
}

#[cfg(windows)]
fn hwnd_from_title(title: &str) -> Option<(windows::Win32::Foundation::HWND, &'static str)> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetForegroundWindow};
    use windows::core::PCWSTR;

    // Primary: exact viewport title passed to `eframe::run_native`.
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    if let Ok(hwnd) = unsafe { FindWindowW(None, PCWSTR(wide.as_ptr())) }
        && hwnd != HWND::default()
    {
        return Some((hwnd, "title"));
    }
    // Fallback: foreground window (usually ours when focused/just launched).
    // Covers cases where the actual viewport title differs from
    // MAIN_VIEWPORT_TITLE (suffixes, eframe renames, etc.).
    let fg = unsafe { GetForegroundWindow() };
    if fg == HWND::default() {
        None
    } else {
        Some((fg, "foreground"))
    }
}

#[cfg(windows)]
/// Resolve the main-viewport HWND and report where it came from
/// (`"frame"`, `"title"`, or `"foreground"`). Logs the source so
/// applied-but-not-visible blur can be traced to a wrong HWND
/// (e.g. foreground fallback pointing at another app).
#[cfg(windows)]
pub fn resolve_hwnd_with_source(
    frame: Option<&eframe::Frame>,
) -> Option<(windows::Win32::Foundation::HWND, &'static str)> {
    if let Some(frame) = frame {
        if let Some(hwnd) = hwnd_from_frame(frame) {
            if glass_debug_enabled() {
                eprintln!("[glass] hwnd resolve source=frame hwnd={hwnd:?}");
            }
            return Some((hwnd, "frame"));
        }
        eprintln!("[glass] hwnd_from_frame failed, trying title/foreground");
    }
    if let Some((hwnd, source)) = hwnd_from_title(MAIN_VIEWPORT_TITLE) {
        if glass_debug_enabled() || source == "foreground" {
            // Always log the foreground fallback: it may belong to a
            // different process, which explains "applied but not visible".
            eprintln!("[glass] hwnd resolve source={source} hwnd={hwnd:?}");
        }
        return Some((hwnd, source));
    }
    None
}

#[cfg(windows)]
fn log_hr(label: &str, code: i32) {
    if code == 0 || glass_debug_enabled() {
        eprintln!("[glass] {label} hr=0x{code:08X} ({code})");
    } else {
        eprintln!("[glass] {label} FAILED hr=0x{code:08X} ({code})");
    }
}

#[cfg(windows)]
pub fn maintain_borderless_window(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::Graphics::Dwm::{
        DWMNCRP_ENABLED, DWMWA_NCRENDERING_POLICY, DwmSetWindowAttribute,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, WS_CAPTION, WS_EX_LAYERED,
        WS_EX_NOREDIRECTIONBITMAP, WS_EX_TRANSPARENT, WS_SYSMENU, WS_THICKFRAME,
    };

    // NOTE: DWMNCRP_USEWINDOWSTYLE (1) tells DWM to skip backdrop rendering
    // for borderless windows, which is exactly the "applied but not visible"
    // symptom: DwmSetWindowAttribute returns S_OK yet no blur appears.
    // Use DWMNCRP_ENABLED (2) so DWM keeps compositing the backdrop behind
    // our transparent egui surface.
    let nc_policy = DWMNCRP_ENABLED.0;
    unsafe {
        match DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY,
            std::ptr::addr_of!(nc_policy).cast::<core::ffi::c_void>(),
            win_cast::size_of_u32::<i32>(),
        ) {
            Ok(()) => log_hr("DWMWA_NCRENDERING_POLICY=ENABLED", 0),
            Err(err) => log_hr("DWMWA_NCRENDERING_POLICY=ENABLED", err.code().0),
        }
    }

    // NOTE: DWMWA_VISIBLE_FRAME_BORDER_THICKNESS is intentionally NOT set.
    // Under DWMNCRP_ENABLED it fails with E_INVALIDARG (0x80070057) and
    // leaves the default border thickness alone. Previous code set it to 0,
    // which only produced a FAILED log line with no visual effect.

    unsafe {
        let style = win_cast::long_ptr_to_u32(GetWindowLongPtrW(hwnd, GWL_STYLE));
        let exstyle = win_cast::long_ptr_to_u32(GetWindowLongPtrW(hwnd, GWL_EXSTYLE));
        // winit `with_transparent(true)` may set WS_EX_NOREDIRECTIONBITMAP
        // and/or WS_EX_LAYERED, both of which disable DWM SYSTEMBACKDROP.
        // Log the extended style every call (cheap, one line in debug) so
        // "all hr=0 but still no blur" can be traced to the compositor path.
        let layered = (exstyle & WS_EX_LAYERED.0) != 0;
        let noredir = (exstyle & WS_EX_NOREDIRECTIONBITMAP.0) != 0;
        let transparent_ex = (exstyle & WS_EX_TRANSPARENT.0) != 0;
        if glass_debug_enabled() {
            eprintln!(
                "[glass] style=0x{style:08X} exstyle=0x{exstyle:08X} layered={layered} noredirectionbitmap={noredir} transparent_ex={transparent_ex}"
            );
        }
        // WS_EX_LAYERED | WS_EX_NOREDIRECTIONBITMAP | WS_EX_TRANSPARENT all
        // disable DWM SYSTEMBACKDROP even when DwmSetWindowAttribute returns
        // S_OK ("applied but not visible"). winit `with_transparent(true)`
        // may set NOREDIRECTIONBITMAP/LAYERED, so clear them here.
        let clear_mask = WS_EX_LAYERED.0 | WS_EX_NOREDIRECTIONBITMAP.0 | WS_EX_TRANSPARENT.0;
        let cleared_exstyle = exstyle & !clear_mask;
        let mut exstyle_changed = false;
        if cleared_exstyle != exstyle {
            SetWindowLongPtrW(
                hwnd,
                GWL_EXSTYLE,
                win_cast::u32_to_long_ptr(cleared_exstyle),
            );
            exstyle_changed = true;
            // Always log when a bit was cleared; before/after in debug mode.
            if glass_debug_enabled() {
                eprintln!(
                    "[glass] exstyle 0x{exstyle:08X} -> 0x{cleared_exstyle:08X} cleared layered/noredir/transparent"
                );
            } else {
                eprintln!(
                    "[glass] cleared WS_EX_LAYERED/NOREDIRECTIONBITMAP/TRANSPARENT exstyle=0x{exstyle:08X} -> 0x{cleared_exstyle:08X}"
                );
            }
        }
        // Keep WS_THICKFRAME: stripping it kills the DWM backdrop, shadow,
        // and rounded corners, and breaks interactive resize. Only strip the
        // caption/symbol chrome (WS_CAPTION | WS_SYSMENU). Re-add THICKFRAME
        // in case winit `with_decorations(false)` removed it.
        let stripped = style & !(WS_CAPTION.0 | WS_SYSMENU.0);
        let desired = stripped | WS_THICKFRAME.0;
        if desired != style || exstyle_changed {
            if desired != style {
                SetWindowLongPtrW(hwnd, GWL_STYLE, win_cast::u32_to_long_ptr(desired));
            }
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
            if glass_debug_enabled() {
                eprintln!("[glass] style 0x{style:08X} -> 0x{desired:08X} FRAMECHANGED");
            }
        }
    }
}

#[cfg(windows)]
/// Apply the DWM backdrop behind the already-transparent egui surface.
///
/// Visual stack (bottom to top): DWM SYSTEMBACKDROP (Acrylic/Mica,
/// best-effort) -> per-pixel alpha (`clear_color()` TRANSPARENT + `shell`
/// TRANSPARENT base + TRANSPARENT board gutters) -> faint readability tint.
/// If every call below returns hr=0 yet no blur is visible, the compositor
/// rejected the backdrop path (e.g. `WS_EX_NOREDIRECTIONBITMAP`/layered in
/// the exstyle log, or a Wgpu swapchain presenting opaque) and the window
/// still composites via per-pixel alpha alone — readable, just not blurred.
/// Try `--renderer=glow` once to isolate a Wgpu alpha-present quirk.
pub fn apply_dwm_glass(hwnd: windows::Win32::Foundation::HWND, backdrop: GlassBackdrop) {
    use windows::Win32::Graphics::Dwm::{
        DWMSBT_MAINWINDOW, DWMSBT_NONE, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE,
        DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
    };
    use windows::Win32::UI::Controls::MARGINS;

    log_windows_build();

    // Must extend the frame over the whole client area BEFORE setting
    // DWMWA_SYSTEMBACKDROP_TYPE, otherwise Acrylic/Mica stays opaque
    // after resize/maximize when DWM resets attributes.
    let margins = MARGINS {
        cxLeftWidth: -1,
        cxRightWidth: -1,
        cyTopHeight: -1,
        cyBottomHeight: -1,
    };
    let extend_result = unsafe { DwmExtendFrameIntoClientArea(hwnd, &raw const margins) };
    match extend_result {
        Ok(()) => log_hr("DwmExtendFrameIntoClientArea(-1)", 0),
        Err(err) => {
            log_hr("DwmExtendFrameIntoClientArea(-1)", err.code().0);
            eprintln!("[glass] DwmExtendFrameIntoClientArea failed: {err:?}");
        }
    }

    let dark_mode: i32 = 1;
    unsafe {
        match DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            std::ptr::addr_of!(dark_mode).cast::<core::ffi::c_void>(),
            win_cast::size_of_u32::<i32>(),
        ) {
            Ok(()) => log_hr("DWMWA_USE_IMMERSIVE_DARK_MODE=1", 0),
            Err(err) => log_hr("DWMWA_USE_IMMERSIVE_DARK_MODE=1", err.code().0),
        }
    }

    let backdrop_type = match backdrop {
        GlassBackdrop::Acrylic => DWMSBT_TRANSIENTWINDOW,
        GlassBackdrop::Mica => DWMSBT_MAINWINDOW,
        GlassBackdrop::None => DWMSBT_NONE,
    };
    let backdrop_value = backdrop_type.0;
    unsafe {
        match DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            std::ptr::addr_of!(backdrop_value).cast::<core::ffi::c_void>(),
            win_cast::size_of_u32::<i32>(),
        ) {
            Ok(()) => log_hr(
                match backdrop {
                    GlassBackdrop::Acrylic => "DWMWA_SYSTEMBACKDROP_TYPE=Acrylic",
                    GlassBackdrop::Mica => "DWMWA_SYSTEMBACKDROP_TYPE=Mica",
                    GlassBackdrop::None => "DWMWA_SYSTEMBACKDROP_TYPE=None",
                },
                0,
            ),
            Err(err) => log_hr(
                match backdrop {
                    GlassBackdrop::Acrylic => "DWMWA_SYSTEMBACKDROP_TYPE=Acrylic",
                    GlassBackdrop::Mica => "DWMWA_SYSTEMBACKDROP_TYPE=Mica",
                    GlassBackdrop::None => "DWMWA_SYSTEMBACKDROP_TYPE=None",
                },
                err.code().0,
            ),
        }
    }

    if backdrop != GlassBackdrop::None {
        let corner = DWMWCP_ROUND.0;
        unsafe {
            match DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                std::ptr::addr_of!(corner).cast::<core::ffi::c_void>(),
                win_cast::size_of_u32::<i32>(),
            ) {
                Ok(()) => log_hr("DWMWA_WINDOW_CORNER_PREFERENCE=ROUND", 0),
                Err(err) => log_hr("DWMWA_WINDOW_CORNER_PREFERENCE=ROUND", err.code().0),
            }
        }
    }
}

#[cfg(windows)]
/// True when `hwnd` belongs to this process. Guards `refresh_glass` so the
/// foreground-HWND fallback never restyles a foreign window (alpha-only
/// compositing fallback instead).
fn hwnd_belongs_to_current_process(hwnd: windows::Win32::Foundation::HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&raw mut pid));
    }
    pid != 0 && pid == std::process::id()
}

#[cfg(windows)]
pub fn refresh_glass(frame: Option<&eframe::Frame>, focused: bool) -> bool {
    let Some((hwnd, source)) = resolve_hwnd_with_source(frame) else {
        eprintln!(
            "[glass] resolve_hwnd failed (frame present: {})",
            frame.is_some()
        );
        return false;
    };
    if !hwnd_belongs_to_current_process(hwnd) {
        eprintln!(
            "[glass] refusing to style foreign hwnd {hwnd:?} via={source} pid mismatch — alpha-only compositing fallback"
        );
        return false;
    }
    maintain_borderless_window(hwnd);
    let backdrop = if focused {
        GlassBackdrop::Acrylic
    } else {
        GlassBackdrop::Mica
    };
    apply_dwm_glass(hwnd, backdrop);
    // Throttle success logs: DWM resets after resize/maximize so we reapply
    // every ~30 frames, but only log when the backdrop mode flips.
    let code: i8 = match backdrop {
        GlassBackdrop::Acrylic => 1,
        GlassBackdrop::Mica => 2,
        GlassBackdrop::None => 0,
    };
    if LAST_LOGGED_BACKDROP.swap(code, std::sync::atomic::Ordering::Relaxed) != code {
        eprintln!("[glass] applied {backdrop:?} focused={focused} hwnd={hwnd:?} via={source}");
    } else if glass_debug_enabled() {
        eprintln!("[glass] reapplied {backdrop:?} focused={focused} hwnd={hwnd:?} via={source}");
    }
    true
}

#[cfg(not(windows))]
pub fn refresh_glass(_frame: Option<&eframe::Frame>, _focused: bool) -> bool {
    false
}
