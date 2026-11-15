//! Windows UI: a small per-pixel-alpha layered window kept on top of the
//! taskbar, just left of the notification area. Standard-user only: no admin,
//! no injection into explorer.exe, no services.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::time::SystemTime;

use windows_sys::core::w;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Accessibility::*;
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::model::{self, Config, Row};

const TIMER_ID: usize = 1;
const TICK_MS: u32 = 500;
const WM_APP_REFRESH: u32 = WM_APP + 1;
const FALLBACK_MARGIN: i32 = 380; // logical px from the taskbar's right edge
const RUN_KEY: *const u16 = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const RUN_VALUE: *const u16 = w!("MinimalClock");

const ID_OPEN: usize = 1;
const ID_AUTO: usize = 2;
const ID_STARTUP: usize = 3;
const ID_EXIT: usize = 9;

struct Theme {
    label: u32,
    time: u32,
    marker: u32,
}
// COLORREF is 0x00BBGGRR.
const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    r | (g << 8) | (b << 16)
}
const DARK: Theme = Theme { label: rgb(0xC4, 0xC4, 0xC4), time: rgb(0xFF, 0xFF, 0xFF), marker: rgb(0xFF, 0xB4, 0x54) };
const LIGHT: Theme = Theme { label: rgb(0x5C, 0x5C, 0x5C), time: rgb(0x1A, 0x1A, 0x1A), marker: rgb(0xA1, 0x4B, 0x00) };

struct State {
    hwnd: HWND,
    cfg_path: PathBuf,
    cfg: Config,
    cfg_mtime: Option<SystemTime>,
    render_key: String,
    pos: (i32, i32, i32, i32),
    visible: bool,
    /// (cursor x at drag start, margin in physical px at drag start)
    drag: Option<(i32, i32)>,
    /// margin in physical px while dragging (not yet saved)
    drag_margin: Option<i32>,
    scale: f64,
    taskbar_created: u32,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static FORCE: Cell<bool> = const { Cell::new(true) };
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn exe_path() -> PathBuf {
    let mut buf = vec![0u16; 1024];
    let n = unsafe { GetModuleFileNameW(null_mut(), buf.as_mut_ptr(), buf.len() as u32) } as usize;
    PathBuf::from(String::from_utf16_lossy(&buf[..n]))
}

fn load_config(path: &PathBuf) -> (Config, Option<SystemTime>) {
    if !path.exists() {
        let _ = std::fs::write(path, model::DEFAULT_CONFIG.replace('\n', "\r\n"));
    }
    let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let cfg = match std::fs::read(path) {
        Ok(bytes) => model::parse_config(&String::from_utf8_lossy(&bytes)),
        Err(_) => Config::default(),
    };
    (cfg, mtime)
}

fn save_margin(path: &PathBuf, margin: Option<i32>) {
    let text = std::fs::read(path).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_else(|_| model::DEFAULT_CONFIG.to_string());
    let value = margin.map(|m| m.to_string()).unwrap_or_default();
    let _ = std::fs::write(path, model::set_key(&text, "right_margin", &value));
}

pub fn run() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        // Single instance per user session.
        let _mutex = CreateMutexW(null(), 0, w!("Local\\MinimalClock.SingleInstance"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }

        let hinst = GetModuleHandleW(null());
        let class = w!("MinimalClockWnd");
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class,
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            class,
            w!("minimal-clock"),
            WS_POPUP,
            0, 0, 1, 1,
            null_mut(),
            null_mut(),
            hinst,
            null(),
        );
        if hwnd.is_null() {
            return;
        }

        let cfg_path = exe_path().with_extension("ini");
        let (cfg, cfg_mtime) = load_config(&cfg_path);
        STATE.with(|s| {
            *s.borrow_mut() = Some(State {
                hwnd,
                cfg_path,
                cfg,
                cfg_mtime,
                render_key: String::new(),
                pos: (0, 0, 0, 0),
                visible: false,
                drag: None,
                drag_margin: None,
                scale: 1.0,
                taskbar_created: RegisterWindowMessageW(w!("TaskbarCreated")),
            })
        });

        // Out-of-context hook (no DLL injection): re-raise ourselves quickly when
        // the foreground window changes, e.g. after the taskbar was clicked.
        SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, null_mut(), Some(win_event), 0, 0, WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS);
        SetTimer(hwnd, TIMER_ID, TICK_MS, None);
        tick();

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn win_event(_: HWINEVENTHOOK, _: u32, _: HWND, _: i32, _: i32, _: u32, _: u32) {
    STATE.with(|s| {
        if let Ok(st) = s.try_borrow() {
            if let Some(st) = st.as_ref() {
                PostMessageW(st.hwnd, WM_APP_REFRESH, 0, 0);
            }
        }
    });
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let taskbar_created = STATE.with(|s| s.try_borrow().ok().and_then(|s| s.as_ref().map(|s| s.taskbar_created)).unwrap_or(0));
    match msg {
        WM_TIMER | WM_APP_REFRESH => {
            tick();
            0
        }
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_LBUTTONDOWN => {
            let mut pt = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            with_state(|st| {
                let margin = current_margin_phys(st);
                st.drag = Some((pt.x, margin));
            });
            SetCapture(hwnd);
            0
        }
        WM_MOUSEMOVE => {
            let mut pt = POINT { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            let moved = with_state(|st| {
                if let Some((x0, m0)) = st.drag {
                    st.drag_margin = Some((m0 - (pt.x - x0)).max(0));
                    true
                } else {
                    false
                }
            });
            if moved == Some(true) {
                tick();
            }
            0
        }
        WM_LBUTTONUP => {
            ReleaseCapture();
            let save = with_state(|st| {
                st.drag = None;
                let m = st.drag_margin.take()?;
                let logical = (m as f64 / st.scale).round() as i32;
                st.cfg.right_margin = Some(logical);
                Some((st.cfg_path.clone(), logical))
            })
            .flatten();
            if let Some((path, logical)) = save {
                save_margin(&path, Some(logical));
                with_state(|st| st.cfg_mtime = std::fs::metadata(&st.cfg_path).and_then(|m| m.modified()).ok());
            }
            0
        }
        WM_RBUTTONUP => {
            show_menu(hwnd);
            0
        }
        WM_SETTINGCHANGE | WM_DISPLAYCHANGE | WM_DPICHANGED | WM_TIMECHANGE | WM_POWERBROADCAST => {
            FORCE.with(|f| f.set(true));
            PostMessageW(hwnd, WM_APP_REFRESH, 0, 0);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ if msg != 0 && msg == taskbar_created => {
            // Explorer restarted: our window survives, just reposition and re-raise.
            FORCE.with(|f| f.set(true));
            PostMessageW(hwnd, WM_APP_REFRESH, 0, 0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|s| s.try_borrow_mut().ok().and_then(|mut s| s.as_mut().map(f)))
}

fn startup_enabled() -> bool {
    unsafe {
        let mut size = 0u32;
        RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE, RRF_RT_REG_SZ, null_mut(), null_mut(), &mut size) == ERROR_SUCCESS
    }
}

fn set_startup(enable: bool) {
    unsafe {
        if enable {
            let cmd = wide(&format!("\"{}\"", exe_path().display()));
            RegSetKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE, REG_SZ, cmd.as_ptr() as _, (cmd.len() * 2) as u32);
        } else {
            RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_VALUE);
        }
    }
}

unsafe fn show_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    AppendMenuW(menu, MF_STRING, ID_OPEN, w!("設定ファイルを開く"));
    AppendMenuW(menu, MF_STRING, ID_AUTO, w!("位置を自動に戻す"));
    let startup = startup_enabled();
    AppendMenuW(menu, MF_STRING | if startup { MF_CHECKED } else { MF_UNCHECKED }, ID_STARTUP, w!("Windows 起動時に開始"));
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(menu, MF_STRING, ID_EXIT, w!("終了"));

    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RIGHTALIGN, pt.x, pt.y, 0, hwnd, null()) as usize;
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);

    match cmd {
        ID_OPEN => {
            if let Some(path) = with_state(|st| st.cfg_path.clone()) {
                let p = wide(&format!("\"{}\"", path.display()));
                ShellExecuteW(null_mut(), w!("open"), w!("notepad.exe"), p.as_ptr(), null(), SW_SHOWNORMAL);
            }
        }
        ID_AUTO => {
            if let Some(path) = with_state(|st| {
                st.cfg.right_margin = None;
                st.cfg_path.clone()
            }) {
                save_margin(&path, None);
                with_state(|st| st.cfg_mtime = std::fs::metadata(&st.cfg_path).and_then(|m| m.modified()).ok());
            }
            FORCE.with(|f| f.set(true));
            tick();
        }
        ID_STARTUP => set_startup(!startup),
        ID_EXIT => {
            DestroyWindow(hwnd);
        }
        _ => {}
    }
}

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

fn window_rect(hwnd: HWND) -> RECT {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    unsafe { GetWindowRect(hwnd, &mut r) };
    r
}

/// Margin from the taskbar's right edge to our right edge, in physical px.
fn current_margin_phys(st: &State) -> i32 {
    if let Some(m) = st.drag_margin {
        return m;
    }
    if let Some(m) = st.cfg.right_margin {
        return (m as f64 * st.scale).round() as i32;
    }
    unsafe {
        let taskbar = FindWindowW(w!("Shell_TrayWnd"), null());
        if !taskbar.is_null() {
            let tb = window_rect(taskbar);
            let tray = FindWindowExW(taskbar, null_mut(), w!("TrayNotifyWnd"), null());
            if !tray.is_null() {
                let tr = window_rect(tray);
                let tb_w = tb.right - tb.left;
                let sane = tr.right > tr.left && tr.left > tb.left + tb_w / 2 && tr.right <= tb.right + 1;
                if sane {
                    return tb.right - tr.left + (4.0 * st.scale).round() as i32;
                }
            }
        }
    }
    (FALLBACK_MARGIN as f64 * st.scale).round() as i32
}

fn foreground_is_fullscreen(mon: &RECT, own: HWND) -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_null() || fg == own {
            return false;
        }
        let cls = class_name(fg);
        if matches!(cls.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
            return false;
        }
        let r = window_rect(fg);
        r.left <= mon.left && r.top <= mon.top && r.right >= mon.right && r.bottom >= mon.bottom
    }
}

fn light_taskbar() -> bool {
    unsafe {
        let mut v: u32 = 0;
        let mut size = 4u32;
        let ok = RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            null_mut(),
            &mut v as *mut u32 as _,
            &mut size,
        );
        ok == ERROR_SUCCESS && v != 0
    }
}

/// True when some window above us in z-order is the taskbar.
fn below_taskbar(own: HWND, taskbar: HWND) -> bool {
    unsafe {
        let mut h = GetWindow(own, GW_HWNDPREV);
        let mut n = 0;
        while !h.is_null() && n < 500 {
            if h == taskbar {
                return true;
            }
            h = GetWindow(h, GW_HWNDPREV);
            n += 1;
        }
        false
    }
}

fn hide(st: &mut State) {
    if st.visible {
        unsafe { ShowWindow(st.hwnd, SW_HIDE) };
        st.visible = false;
    }
}

fn tick() {
    let force = FORCE.with(|f| f.replace(false));
    let _ = with_state(|st| unsafe { tick_inner(st, force) });
}

unsafe fn tick_inner(st: &mut State, force: bool) {
    // Pick up edits of the settings file.
    let mtime = std::fs::metadata(&st.cfg_path).and_then(|m| m.modified()).ok();
    if mtime != st.cfg_mtime {
        let (cfg, m) = load_config(&st.cfg_path);
        st.cfg = cfg;
        st.cfg_mtime = m;
    }
    if force {
        st.render_key.clear();
    }

    let taskbar = FindWindowW(w!("Shell_TrayWnd"), null());
    if taskbar.is_null() {
        return hide(st);
    }
    let tb = window_rect(taskbar);
    let mut mi: MONITORINFO = std::mem::zeroed();
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    GetMonitorInfoW(MonitorFromWindow(taskbar, MONITOR_DEFAULTTOPRIMARY), &mut mi);
    let mon = mi.rcMonitor;

    // Visible part of the (horizontal) taskbar on its monitor.
    let top = tb.top.max(mon.top);
    let bottom = tb.bottom.min(mon.bottom);
    let tb_h = tb.bottom - tb.top;
    let horizontal = tb.right - tb.left > tb_h;
    if !horizontal || bottom - top < tb_h / 2 || foreground_is_fullscreen(&mon, st.hwnd) {
        return hide(st);
    }

    let dpi = GetDpiForWindow(taskbar);
    st.scale = if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 };
    let light = light_taskbar();

    let now = chrono::Utc::now();
    let pc_date = chrono::Local::now().date_naive();
    let rows = model::rows(&st.cfg, now, pc_date);
    let h = bottom - top;

    let key = format!("{rows:?}|{light}|{dpi}|{h}");
    let mut width = st.pos.2;
    let mut bitmap = None;
    if key != st.render_key || width == 0 {
        let (w, bits) = render(&rows, if light { &LIGHT } else { &DARK }, st.scale, h);
        width = w;
        bitmap = Some(bits);
        st.render_key = key;
    }

    let margin = current_margin_phys(st).clamp(0, (tb.right - tb.left - width).max(0));
    let x = tb.right - margin - width;
    let pos = (x, top, width, h);

    if let Some(bits) = bitmap {
        present(st.hwnd, &bits, pos);
    } else if pos != st.pos {
        SetWindowPos(st.hwnd, HWND_TOPMOST, x, top, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
    }
    st.pos = pos;

    if !st.visible {
        ShowWindow(st.hwnd, SW_SHOWNOACTIVATE);
        st.visible = true;
    }
    if below_taskbar(st.hwnd, taskbar) {
        SetWindowPos(st.hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

unsafe fn make_font(px: f64, weight: i32) -> HFONT {
    CreateFontW(
        -(px.round() as i32),
        0, 0, 0,
        weight,
        0, 0, 0,
        DEFAULT_CHARSET as _,
        OUT_DEFAULT_PRECIS as _,
        CLIP_DEFAULT_PRECIS as _,
        ANTIALIASED_QUALITY as _, // grayscale AA: ClearType would fringe on a transparent bitmap
        (DEFAULT_PITCH | FF_DONTCARE) as _,
        w!("Segoe UI"),
    )
}

unsafe fn text_width(dc: HDC, s: &str) -> i32 {
    if s.is_empty() {
        return 0;
    }
    let ws: Vec<u16> = s.encode_utf16().collect();
    let mut size = SIZE { cx: 0, cy: 0 };
    GetTextExtentPoint32W(dc, ws.as_ptr(), ws.len() as i32, &mut size);
    size.cx
}

unsafe fn text_out(dc: HDC, x: i32, y: i32, s: &str) {
    let ws: Vec<u16> = s.encode_utf16().collect();
    TextOutW(dc, x, y, ws.as_ptr(), ws.len() as i32);
}

/// Renders the rows into premultiplied BGRA pixels. Returns (width, pixels).
unsafe fn render(rows: &[Row], theme: &Theme, scale: f64, h: i32) -> (i32, Vec<u32>) {
    let s = |v: f64| (v * scale).round() as i32;
    let f_label = make_font(12.0 * scale, FW_NORMAL as i32);
    let f_time = make_font(12.0 * scale, FW_SEMIBOLD as i32);
    let f_mark = make_font(11.0 * scale, FW_BOLD as i32);

    let screen = GetDC(null_mut());
    let dc = CreateCompatibleDC(screen);

    // Measure columns.
    SelectObject(dc, f_label);
    let label_w = rows.iter().map(|r| text_width(dc, &r.label)).max().unwrap_or(0);
    SelectObject(dc, f_time);
    let time_w = text_width(dc, "00:00").max(rows.iter().map(|r| text_width(dc, &r.time)).max().unwrap_or(0));
    let mut tm: TEXTMETRICW = std::mem::zeroed();
    GetTextMetricsW(dc, &mut tm);
    SelectObject(dc, f_mark);
    let mark_w = text_width(dc, "\u{2212}1").max(text_width(dc, "+1")).max(rows.iter().map(|r| text_width(dc, &r.marker)).max().unwrap_or(0));

    let pad = s(8.0);
    let gap = s(5.0);
    let x_label = pad;
    let x_time = x_label + label_w + gap;
    let x_mark = x_time + time_w + gap;
    let width = x_mark + mark_w + pad;

    let line_h = s(15.0);
    let block = line_h * rows.len() as i32;
    let y0 = (h - block) / 2;
    let text_h = tm.tmAscent + tm.tmDescent;
    let baseline = |i: usize| y0 + line_h * i as i32 + (line_h - text_h) / 2 + tm.tmAscent;

    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = width;
    bmi.bmiHeader.biHeight = -h; // top-down
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB as u32;
    let mut bits_ptr: *mut core::ffi::c_void = null_mut();
    let bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, null_mut(), 0);
    let old_bmp = SelectObject(dc, bmp);
    let n = (width * h) as usize;
    let bits = std::slice::from_raw_parts_mut(bits_ptr as *mut u32, n);

    SetBkMode(dc, TRANSPARENT as _);
    SetTextColor(dc, rgb(255, 255, 255));
    SetTextAlign(dc, TA_BASELINE | TA_LEFT);

    // Alpha 1 everywhere so the whole rectangle receives mouse input
    // (fully transparent pixels of a layered window are click-through).
    let mut out = vec![0x0100_0000u32; n];

    // One pass per colour: draw white on black, use the grey level as coverage.
    let layers: [(HFONT, i32, u32, fn(&Row) -> &str); 3] = [
        (f_label, x_label, theme.label, |r| r.label.as_str()),
        (f_time, x_time, theme.time, |r| r.time.as_str()),
        (f_mark, x_mark, theme.marker, |r| r.marker.as_str()),
    ];
    for (font, x, color, pick) in layers {
        bits.fill(0);
        SelectObject(dc, font);
        for (i, r) in rows.iter().enumerate() {
            text_out(dc, x, baseline(i), pick(r));
        }
        GdiFlush();
        let (cr, cg, cb) = (color & 0xFF, (color >> 8) & 0xFF, (color >> 16) & 0xFF);
        for (o, &p) in out.iter_mut().zip(bits.iter()) {
            let a = (p >> 8) & 0xFF; // green channel = coverage
            if a > 1 {
                let pm = |c: u32| c * a / 255;
                *o = (a << 24) | (pm(cr) << 16) | (pm(cg) << 8) | pm(cb);
            }
        }
    }

    SelectObject(dc, old_bmp);
    DeleteObject(bmp);
    DeleteDC(dc);
    ReleaseDC(null_mut(), screen);
    DeleteObject(f_label);
    DeleteObject(f_time);
    DeleteObject(f_mark);
    (width, out)
}

/// Pushes premultiplied pixels to the layered window at the given position.
unsafe fn present(hwnd: HWND, pixels: &[u32], (x, y, w, h): (i32, i32, i32, i32)) {
    let screen = GetDC(null_mut());
    let dc = CreateCompatibleDC(screen);
    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = w;
    bmi.bmiHeader.biHeight = -h;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB as u32;
    let mut bits_ptr: *mut core::ffi::c_void = null_mut();
    let bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, null_mut(), 0);
    if !bmp.is_null() {
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits_ptr as *mut u32, pixels.len());
        let old = SelectObject(dc, bmp);
        let dst = POINT { x, y };
        let size = SIZE { cx: w, cy: h };
        let src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
        UpdateLayeredWindow(hwnd, screen, &dst, &size, dc, &src, 0, &blend, ULW_ALPHA);
        SelectObject(dc, old);
        DeleteObject(bmp);
    }
    DeleteDC(dc);
    ReleaseDC(null_mut(), screen);
    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
}
