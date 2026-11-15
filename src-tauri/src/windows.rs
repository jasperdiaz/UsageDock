use crate::settings::DOCKS;
use crate::{AppState, tray};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

pub const WIDGET: &str = "widget";
pub const DASHBOARD: &str = "dashboard";

/// Starting size before the UI reports its real size (it sizes itself to content).
pub const WIDGET_DEFAULT: (f64, f64) = (200.0, 120.0);
pub const DASHBOARD_SIZE: (f64, f64) = (380.0, 480.0);
const EDGE_MARGIN: i32 = 12;
/// How close (logical px) a dragged widget must land to a dock spot to snap to it.
const SNAP_DISTANCE: f64 = 72.0;

/// The last position we set ourselves, so Moved events from it aren't taken as a user drag.
static LAST_SET: Mutex<Option<(i32, i32)>> = Mutex::new(None);
/// Bumped on every user move; a drag is "finished" once it stops changing.
static MOVE_SEQ: AtomicU64 = AtomicU64::new(0);
/// Transparent room (logical px) the UI keeps around the panel for its glow.
/// Docking subtracts it so the visible panel keeps the usual edge margin.
static INSET: Mutex<f64> = Mutex::new(0.0);
/// Launch sequence: the widget sits centred behind its splash screen until the
/// UI calls `finish_launch`, then glides to where it belongs.
static LAUNCHING: AtomicBool = AtomicBool::new(true);
/// True while the launch glide is moving the window.
static GLIDING: AtomicBool = AtomicBool::new(false);
const GLIDE_MS: u64 = 1100;
/// If the UI never reports in, land the widget anyway.
const LAUNCH_FALLBACK: Duration = Duration::from_secs(8);

fn set_pos(w: &WebviewWindow, x: i32, y: i32) {
    *LAST_SET.lock().unwrap() = Some((x, y));
    let _ = w.set_position(PhysicalPosition::new(x, y));
}

fn build(app: &AppHandle, label: &str, size: (f64, f64), url: &str) -> tauri::Result<WebviewWindow> {
    let st = app.state::<AppState>();
    let s = st.settings.lock().unwrap().clone();
    let mut b = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
        .title("UsageDock")
        .inner_size(size.0, size.1)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(s.show_in_tray)
        .always_on_top(s.always_on_top)
        .visible(false)
        .focused(false);
    b = if label == WIDGET {
        b.resizable(false)
    } else {
        b.min_inner_size(340.0, 420.0).resizable(true)
    };
    b.build()
}

pub fn create_all(app: &AppHandle) -> tauri::Result<()> {
    let st = app.state::<AppState>();
    let s = st.settings.lock().unwrap().clone();
    let widget_size = s.widget_size.unwrap_or(WIDGET_DEFAULT);
    let widget = build(app, WIDGET, widget_size, "index.html")?;
    center(&widget);
    attach_events(app, &widget);
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(LAUNCH_FALLBACK);
        finish_launch(&handle, false);
    });
    Ok(())
}

fn attach_events(app: &AppHandle, w: &WebviewWindow) {
    let app = app.clone();
    let label = w.label().to_string();
    w.on_window_event(move |e| on_event(&app, &label, e));
}

/// The dashboard is created on first use so a widget-only or tray-only session
/// runs a single WebView renderer. Once created it is kept and just hidden.
pub fn dashboard(app: &AppHandle, tab: Option<&str>) -> Option<WebviewWindow> {
    if let Some(w) = app.get_webview_window(DASHBOARD) {
        return Some(w);
    }
    let st = app.state::<AppState>();
    let saved = st.settings.lock().unwrap().dashboard_position;
    let url = format!("index.html?tab={}", tab.unwrap_or("usage"));
    let w = build(app, DASHBOARD, DASHBOARD_SIZE, &url).ok()?;
    place(&w, saved);
    attach_events(app, &w);
    Some(w)
}

/// Restore a saved position only if it's still on a connected monitor
/// (monitors get unplugged and DPI changes); otherwise go bottom-right.
fn place(w: &WebviewWindow, saved: Option<(i32, i32)>) {
    if let Some((x, y)) = saved {
        let visible = w
            .available_monitors()
            .unwrap_or_default()
            .iter()
            .any(|m| contains(m, x + 40, y + 20));
        if visible {
            let _ = w.set_position(PhysicalPosition::new(x, y));
            return;
        }
    }
    place_default(w);
}

fn contains(m: &Monitor, x: i32, y: i32) -> bool {
    let p = m.position();
    let s = m.size();
    x >= p.x && y >= p.y && x < p.x + s.width as i32 && y < p.y + s.height as i32
}

fn work_area(m: &Monitor) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    let wa = m.work_area();
    (wa.position, wa.size)
}

/// The monitor the window is on, else the primary one.
fn monitor_of(w: &WebviewWindow) -> Option<Monitor> {
    w.current_monitor().ok().flatten().or_else(|| w.primary_monitor().ok().flatten())
}

/// Top-left corner for `size` at a dock spot on `m`'s work area.
fn dock_point(m: &Monitor, dock: &str, size: PhysicalSize<u32>) -> (i32, i32) {
    let (pos, area) = work_area(m);
    let inset = *INSET.lock().unwrap();
    let margin = ((EDGE_MARGIN as f64 - inset) * m.scale_factor()) as i32;
    let (w, h) = (size.width as i32, size.height as i32);
    let (aw, ah) = (area.width as i32, area.height as i32);
    let left = pos.x + margin;
    let right = pos.x + aw - w - margin;
    let cx = pos.x + (aw - w) / 2;
    let top = pos.y + margin;
    let bottom = pos.y + ah - h - margin;
    let cy = pos.y + (ah - h) / 2;
    match dock {
        "top-left" => (left, top),
        "top-center" => (cx, top),
        "top-right" => (right, top),
        "left-center" => (left, cy),
        "right-center" => (right, cy),
        "bottom-left" => (left, bottom),
        "bottom-center" => (cx, bottom),
        _ => (right, bottom),
    }
}

/// Move the widget to its dock spot (or keep it on-screen when free).
pub fn apply_dock(app: &AppHandle) {
    let Some(w) = app.get_webview_window(WIDGET) else { return };
    if LAUNCHING.load(Ordering::SeqCst) {
        center(&w);
        return;
    }
    // The glide re-docks when it lands, using the size at that moment.
    if GLIDING.load(Ordering::SeqCst) {
        return;
    }
    let dock = app.state::<AppState>().settings.lock().unwrap().dock.clone();
    let (Some(m), Ok(size)) = (monitor_of(&w), w.outer_size()) else { return };
    if dock == "free" {
        // Keep the panel fully on its monitor after a size change. The glow
        // room around it may hang off the edge.
        let Ok(p) = w.outer_position() else { return };
        let (pos, area) = work_area(&m);
        let slack = (*INSET.lock().unwrap() * m.scale_factor()) as i32;
        let x = p.x.clamp(pos.x - slack, (pos.x + area.width as i32 - size.width as i32 + slack).max(pos.x - slack));
        let y = p.y.clamp(pos.y - slack, (pos.y + area.height as i32 - size.height as i32 + slack).max(pos.y - slack));
        if (x, y) != (p.x, p.y) {
            set_pos(&w, x, y);
        }
        return;
    }
    let (x, y) = dock_point(&m, &dock, size);
    set_pos(&w, x, y);
}

/// The UI measured its content: resize the widget to it and re-anchor.
pub fn fit_widget(app: &AppHandle, width: f64, height: f64, inset: f64) {
    let Some(w) = app.get_webview_window(WIDGET) else { return };
    let inset = inset.clamp(0.0, 40.0);
    let prev = std::mem::replace(&mut *INSET.lock().unwrap(), inset);
    let (width, height) = (width.clamp(80.0, 1600.0).ceil(), height.clamp(32.0, 1200.0).ceil());
    let _ = w.set_size(LogicalSize::new(width, height));
    // A free-placed widget keeps its panel where it is: when the glow room
    // grows or shrinks, move the window by the difference.
    let free = app.state::<AppState>().settings.lock().unwrap().dock == "free";
    if free && (inset - prev).abs() > f64::EPSILON {
        if let Ok(p) = w.outer_position() {
            let d = ((inset - prev) * w.scale_factor().unwrap_or(1.0)).round() as i32;
            set_pos(&w, p.x - d, p.y - d);
        }
    }
    {
        let st = app.state::<AppState>();
        let mut s = st.settings.lock().unwrap();
        if s.widget_size != Some((width, height)) {
            s.widget_size = Some((width, height));
            st.mark_dirty();
        }
    }
    apply_dock(app);
}

/// Centre the widget on the primary monitor's work area (the splash spot).
fn center(w: &WebviewWindow) {
    let Some(m) = w.primary_monitor().ok().flatten().or_else(|| monitor_of(w)) else { return };
    let Ok(size) = w.outer_size() else { return };
    let (pos, area) = work_area(&m);
    let x = pos.x + (area.width as i32 - size.width as i32) / 2;
    let y = pos.y + (area.height as i32 - size.height as i32) / 2;
    set_pos(w, x, y);
}

/// Where the widget belongs: its dock spot, or its saved free position if
/// that is still on a connected monitor, else bottom-right.
fn home_point(app: &AppHandle, w: &WebviewWindow) -> Option<(i32, i32)> {
    let s = app.state::<AppState>().settings.lock().unwrap().clone();
    let size = w.outer_size().ok()?;
    if s.dock == "free" {
        if let Some((x, y)) = s.widget_position {
            let on_screen = w.available_monitors().unwrap_or_default().iter().any(|m| contains(m, x + 40, y + 20));
            if on_screen {
                return Some((x, y));
            }
        }
        let m = w.primary_monitor().ok().flatten()?;
        return Some(dock_point(&m, "bottom-right", size));
    }
    let m = w.primary_monitor().ok().flatten().or_else(|| monitor_of(w))?;
    Some(dock_point(&m, &s.dock, size))
}

/// Smooth in-and-out travel: eases off the splash spot and settles into place.
fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 { 4.0 * t * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(3) / 2.0 }
}

/// End the launch: glide the widget from the centre to its home. Runs once;
/// later calls (and the fallback timer) do nothing.
pub fn finish_launch(app: &AppHandle, animate: bool) {
    if !LAUNCHING.swap(false, Ordering::SeqCst) {
        return;
    }
    let Some(w) = app.get_webview_window(WIDGET) else { return };
    let (Ok(from), Some(to)) = (w.outer_position(), home_point(app, &w)) else {
        apply_dock(app);
        return;
    };
    if !animate || !w.is_visible().unwrap_or(false) {
        set_pos(&w, to.0, to.1);
        apply_dock(app);
        return;
    }
    GLIDING.store(true, Ordering::SeqCst);
    let app = app.clone();
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        let total = Duration::from_millis(GLIDE_MS);
        loop {
            let t = (start.elapsed().as_secs_f64() / total.as_secs_f64()).min(1.0);
            let e = ease_in_out_cubic(t);
            let x = from.x + ((to.0 - from.x) as f64 * e).round() as i32;
            let y = from.y + ((to.1 - from.y) as f64 * e).round() as i32;
            set_pos(&w, x, y);
            if t >= 1.0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(8));
        }
        GLIDING.store(false, Ordering::SeqCst);
        // Docked: re-anchor with the final size. Free: keep the landing spot.
        apply_dock(&app);
    });
}

pub fn set_dock(app: &AppHandle, dock: &str) {
    if dock != "free" && !DOCKS.contains(&dock) {
        return;
    }
    {
        let st = app.state::<AppState>();
        st.settings.lock().unwrap().dock = dock.to_string();
        st.mark_dirty();
    }
    apply_dock(app);
    let s = app.state::<AppState>().settings.lock().unwrap().clone();
    let _ = app.emit("settings://changed", &s);
}

/// After a drag settles: snap to the nearest dock spot if close, else stay free.
fn settle_drag(app: &AppHandle) {
    let Some(w) = app.get_webview_window(WIDGET) else { return };
    let (Some(m), Ok(size), Ok(p)) = (monitor_of(&w), w.outer_size(), w.outer_position()) else { return };
    let limit = SNAP_DISTANCE * m.scale_factor();
    let nearest = DOCKS
        .iter()
        .map(|d| {
            let (x, y) = dock_point(&m, d, size);
            (*d, (((x - p.x) as f64).powi(2) + ((y - p.y) as f64).powi(2)).sqrt())
        })
        .min_by(|a, b| a.1.total_cmp(&b.1));
    match nearest {
        Some((d, dist)) if dist <= limit => set_dock(app, d),
        _ => set_dock(app, "free"),
    }
}

pub fn place_default(w: &WebviewWindow) {
    let Ok(Some(m)) = w.primary_monitor() else { return };
    let (pos, size) = work_area(&m);
    let Ok(win) = w.outer_size() else { return };
    let margin = (EDGE_MARGIN as f64 * m.scale_factor()) as i32;
    let mut x = pos.x + size.width as i32 - win.width as i32 - margin;
    let y = pos.y + size.height as i32 - win.height as i32 - margin;
    if w.label() == DASHBOARD {
        // Sit to the left of the widget's default spot.
        x -= (WIDGET_DEFAULT.0 * m.scale_factor()) as i32 + margin;
    }
    let _ = w.set_position(PhysicalPosition::new(x.max(pos.x), y.max(pos.y)));
}

/// Position the dashboard like a tray flyout: centred on the icon, on the
/// taskbar side, clamped to the work area of the icon's monitor.
pub fn place_near_tray(w: &WebviewWindow, tray_pos: PhysicalPosition<i32>, tray_size: PhysicalSize<u32>) {
    let Ok(Some(m)) = w.app_handle().monitor_from_point(tray_pos.x as f64, tray_pos.y as f64) else {
        return;
    };
    let (pos, size) = work_area(&m);
    let Ok(win) = w.outer_size() else { return };
    let gap = (EDGE_MARGIN as f64 * m.scale_factor()) as i32;
    let cx = tray_pos.x + tray_size.width as i32 / 2;
    let mut x = cx - win.width as i32 / 2;
    let taskbar_top = tray_pos.y < pos.y + size.height as i32 / 2;
    let mut y = if taskbar_top {
        pos.y + gap
    } else {
        pos.y + size.height as i32 - win.height as i32 - gap
    };
    x = x.clamp(pos.x + gap, (pos.x + size.width as i32 - win.width as i32 - gap).max(pos.x));
    y = y.max(pos.y);
    let _ = w.set_position(PhysicalPosition::new(x, y));
}

fn on_event(app: &AppHandle, label: &str, e: &WindowEvent) {
    let st = app.state::<AppState>();
    match e {
        WindowEvent::CloseRequested { api, .. } => {
            if !st.quitting.load(std::sync::atomic::Ordering::SeqCst) {
                api.prevent_close();
                hide_or_quit(app, label);
            }
        }
        WindowEvent::Moved(p) => {
            // Launch moves are ours: don't save them or read them as a drag.
            if label == WIDGET && (LAUNCHING.load(Ordering::SeqCst) || GLIDING.load(Ordering::SeqCst)) {
                return;
            }
            let Some(w) = app.get_webview_window(label) else { return };
            // Moved also fires for minimise/hidden states with bogus coordinates.
            if !w.is_visible().unwrap_or(false) || w.is_minimized().unwrap_or(false) {
                return;
            }
            {
                let mut s = st.settings.lock().unwrap();
                if label == WIDGET {
                    s.widget_position = Some((p.x, p.y));
                } else {
                    s.dashboard_position = Some((p.x, p.y));
                }
                st.mark_dirty();
            }
            // A move we didn't make is the user dragging the widget.
            if label == WIDGET && *LAST_SET.lock().unwrap() != Some((p.x, p.y)) {
                let seq = MOVE_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
                let app = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(500));
                    if MOVE_SEQ.load(Ordering::SeqCst) == seq {
                        settle_drag(&app);
                    }
                });
            }
        }
        _ => {}
    }
}

pub fn show(app: &AppHandle, label: &str) {
    if label == DASHBOARD {
        dashboard(app, None);
    }
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
    tray::sync_menu(app);
}

pub fn show_dashboard(app: &AppHandle, tab: Option<&str>) {
    dashboard(app, tab);
    show(app, DASHBOARD);
    if let Some(tab) = tab {
        let _ = app.emit_to(DASHBOARD, "dashboard://tab", tab);
    }
}

/// Hide to the tray, or quit if there's no tray icon and nothing else is open.
pub fn hide_or_quit(app: &AppHandle, label: &str) {
    let st = app.state::<AppState>();
    let show_in_tray = st.settings.lock().unwrap().show_in_tray;
    let other = if label == WIDGET { DASHBOARD } else { WIDGET };
    let other_visible = app
        .get_webview_window(other)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    if !show_in_tray && !other_visible {
        crate::quit(app);
        return;
    }
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.hide();
    }
    tray::sync_menu(app);
}

pub fn is_visible(app: &AppHandle, label: &str) -> bool {
    app.get_webview_window(label)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

pub fn apply_window_flags(app: &AppHandle) {
    let st = app.state::<AppState>();
    let s = st.settings.lock().unwrap().clone();
    for label in [WIDGET, DASHBOARD] {
        if let Some(w) = app.get_webview_window(label) {
            let _ = w.set_always_on_top(s.always_on_top);
            let _ = w.set_skip_taskbar(s.show_in_tray);
        }
    }
}

/// The UI re-measures and calls `fit_widget` after the change.
pub fn set_widget_collapsed(app: &AppHandle, collapsed: bool) {
    let st = app.state::<AppState>();
    st.settings.lock().unwrap().widget_collapsed = collapsed;
    st.mark_dirty();
}

pub fn reset_positions(app: &AppHandle) {
    let st = app.state::<AppState>();
    {
        let mut s = st.settings.lock().unwrap();
        s.widget_position = None;
        s.dashboard_position = None;
        s.widget_size = None;
        s.widget_collapsed = false;
        s.dock = "bottom-right".into();
        st.mark_dirty();
    }
    apply_dock(app);
    if let Some(w) = app.get_webview_window(DASHBOARD) {
        let _ = w.set_size(LogicalSize::new(DASHBOARD_SIZE.0, DASHBOARD_SIZE.1));
        place_default(&w);
    }
}
