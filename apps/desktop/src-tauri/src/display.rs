//! The screens, the way each system reports them: which one the slime lives on, its play area, and what to do when the
//! setup changes while the app runs (a screen plugged in, a resolution or scale change, a bar appearing). The rules are in
//! `slimy-display`; this file only reads the system and moves the window.

use std::path::Path;
use std::time::{Duration, Instant};

use slimy_display::{
    choose, screen_at, signature, size_scale, world_of, Follower, Preference, Rect, Screen, World,
};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

const FILE: &str = "display.json";

pub fn load_preference(dir: &Path) -> Preference {
    std::fs::read_to_string(dir.join(FILE))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| {
            v.get("monitor")
                .and_then(|m| m.as_str().map(Preference::parse))
        })
        .unwrap_or_default()
}

pub fn save_preference(dir: &Path, preference: Preference) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let staging = dir.join("display.json.tmp");
    std::fs::write(
        &staging,
        serde_json::json!({ "monitor": preference.text() }).to_string(),
    )?;
    std::fs::rename(staging, dir.join(FILE))
}

/// Every screen the system reports, with its work area.
pub fn screens(window: &WebviewWindow) -> Vec<Screen> {
    let primary = window.primary_monitor().ok().flatten();
    let mut out: Vec<Screen> = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let (p, s, w) = (m.position(), m.size(), m.work_area());
            Screen {
                rect: Rect {
                    x: p.x,
                    y: p.y,
                    w: s.width,
                    h: s.height,
                },
                work: Rect {
                    x: w.position.x,
                    y: w.position.y,
                    w: w.size.width,
                    h: w.size.height,
                },
                scale: m.scale_factor(),
                primary: primary
                    .as_ref()
                    .is_some_and(|pm| pm.position() == m.position() && pm.size() == m.size()),
            }
        })
        .collect();
    if out.is_empty() {
        // The system reported nothing: assume a plain 1080p screen rather than refusing to start.
        let full = Rect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        };
        out.push(Screen {
            rect: full,
            work: full,
            scale: 1.0,
            primary: true,
        });
    }
    #[cfg(target_os = "linux")]
    for screen in out.iter_mut().filter(|s| s.primary && s.work == s.rect) {
        // The window system's own work area knows about the shell's bar when the toolkit does not.
        if let Some(area) = crate::x11_work_area() {
            let refined = Rect {
                x: area.left as i32,
                y: area.top as i32,
                w: (area.right - area.left) as u32,
                h: (area.bottom - area.top) as u32,
            };
            let (right, bottom) = (
                i64::from(refined.x) + i64::from(refined.w),
                i64::from(refined.y) + i64::from(refined.h),
            );
            if refined.x >= screen.rect.x
                && refined.y >= screen.rect.y
                && right <= screen.rect.x as i64 + screen.rect.w as i64
                && bottom <= screen.rect.y as i64 + screen.rect.h as i64
                && refined.w > 0
                && refined.h > 0
            {
                screen.work = refined;
            }
        }
    }
    out
}

/// Where the slime lives and what the page needs to size itself.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Layout {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    /// Scale factor of the chosen screen.
    pub scale_factor: f64,
    /// How big the slime should be here, as a multiplier of its usual size.
    pub size_scale: f32,
    pub index: usize,
    pub count: usize,
    pub preference: String,
}

pub fn layout_for(screens: &[Screen], index: usize, preference: Preference) -> Layout {
    let index = index.min(screens.len().saturating_sub(1));
    let screen = &screens[index];
    let world: World = world_of(screen);
    let logical_height = (world.bottom - world.top) / screen.scale.max(0.1);
    Layout {
        left: world.left,
        top: world.top,
        right: world.right,
        bottom: world.bottom,
        scale_factor: screen.scale,
        size_scale: size_scale(logical_height),
        index,
        count: screens.len(),
        preference: preference.text(),
    }
}

/// The pointer in desktop pixels, when the system lets us read it.
fn pointer(window: &WebviewWindow) -> Option<(f64, f64)> {
    crate::read_pointer_position(window.clone())
}

/// The layout right now: the preferred screen (or the one under the pointer when following).
pub fn current(window: &WebviewWindow, dir: &Path) -> Layout {
    let screens = screens(window);
    let preference = load_preference(dir);
    let at = if preference == Preference::Follow {
        pointer(window)
    } else {
        None
    };
    let index = choose(&screens, preference, at, None);
    layout_for(&screens, index, preference)
}

/// Puts the window exactly over a play area.
pub fn cover(window: &WebviewWindow, layout: &Layout) -> Result<(), String> {
    window
        .set_size(PhysicalSize::new(
            (layout.right - layout.left) as u32,
            (layout.bottom - layout.top) as u32,
        ))
        .map_err(|e| e.to_string())?;
    window
        .set_position(PhysicalPosition::new(layout.left as i32, layout.top as i32))
        .map_err(|e| e.to_string())
}

/// Watches the setup while the app runs: when it changes (or the user changes the preferred screen, or the pointer rests on
/// another screen while following) the window is moved over the new play area and the page is told with `world-changed`.
pub fn watch(app: tauri::AppHandle, dir: std::path::PathBuf) {
    std::thread::spawn(move || {
        let started = Instant::now();
        let mut follower = Follower::default();
        let mut current_index: Option<usize> = None;
        let mut last: Option<u64> = None;
        loop {
            std::thread::sleep(Duration::from_millis(1500));
            let Some(window) = app.get_webview_window("main") else {
                continue;
            };
            let screens = screens(&window);
            let preference = load_preference(&dir);
            let index = if preference == Preference::Follow {
                let here = current_index
                    .unwrap_or_else(|| choose(&screens, preference, pointer(&window), None))
                    .min(screens.len() - 1);
                let under = pointer(&window).and_then(|(x, y)| screen_at(&screens, x, y));
                follower.update(started.elapsed().as_secs_f64(), here, under)
            } else {
                choose(&screens, preference, None, current_index)
            };
            current_index = Some(index);
            let layout = layout_for(&screens, index, preference);
            let sign = signature(&screens)
                ^ (index as u64).wrapping_mul(0x9E37_79B9)
                ^ u64::from(preference == Preference::Follow);
            match last {
                // the first look only records what the page already fetched at start
                None => last = Some(sign),
                Some(previous) if previous != sign => {
                    last = Some(sign);
                    if cover(&window, &layout).is_ok() {
                        let _ = app.emit("world-changed", &layout);
                    }
                }
                _ => {}
            }
        }
    });
}

/// Where clicks must be taken, kept for the click-through loop on systems that cannot shape the input region.
#[cfg(not(target_os = "linux"))]
pub static HIT: std::sync::Mutex<slimy_display::HitRegion> =
    std::sync::Mutex::new(slimy_display::HitRegion {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    });

/// Windows and macOS cannot cut the input region of a window like X11 can: the whole window is switched between taking
/// and ignoring the mouse, following where the pointer is. Without this the transparent window would swallow every click.
#[cfg(not(target_os = "linux"))]
pub fn click_through(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut gate = slimy_display::ClickThrough::default();
        loop {
            std::thread::sleep(Duration::from_millis(25));
            let Some(window) = app.get_webview_window("main") else {
                continue;
            };
            let region = *HIT.lock().unwrap_or_else(|e| e.into_inner());
            let scale = window.scale_factor().unwrap_or(1.0);
            let local = match (window.cursor_position(), window.inner_position()) {
                (Ok(c), Ok(i)) => Some((c.x - f64::from(i.x), c.y - f64::from(i.y))),
                _ => None,
            };
            if let Some(ignore) = gate.update(&region.scaled(scale), local) {
                let _ = window.set_ignore_cursor_events(ignore);
            }
        }
    });
}
