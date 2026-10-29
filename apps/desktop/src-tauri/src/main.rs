mod brain;
mod clipboard;
mod display;
#[cfg(all(debug_assertions, target_os = "linux"))]
mod drive;
mod files;
mod learn;
mod mcp;
mod perms;
mod reminders;
mod remote;
mod screen;
mod secrets;
mod skills;
mod slimefile;
mod store;
mod voice;

use std::process::Command;

use serde::Serialize;
use slimy_core::{DayLog, Genome, Soul, SoulError};
use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

/// Area of the screen the slime may roam, in physical pixels.
#[derive(Serialize, serde::Deserialize)]
struct World {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

/// `_NET_WORKAREA` reports the usable area (below the GNOME bar); Tauri's `work_area()` is unreliable on X11.
pub(crate) fn x11_work_area() -> Option<World> {
    let out = Command::new("xprop")
        .args(["-root", "_NET_WORKAREA"])
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let values: Vec<f64> = text
        .split('=')
        .nth(1)?
        .split(',')
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    let [x, y, w, h, ..] = values[..] else {
        return None;
    };
    Some(World {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    })
}

/// Where the slime lives (the play area in physical pixels, plus the scale and the size it should have here).
#[tauri::command]
fn world(app: tauri::AppHandle, window: WebviewWindow) -> Result<display::Layout, String> {
    Ok(display::current(&window, &soul_dir(&app)?))
}

/// The screens and which one the slime lives on.
#[tauri::command]
fn display_status(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<serde_json::Value, String> {
    let dir = soul_dir(&app)?;
    let screens = display::screens(&window);
    let layout = display::current(&window, &dir);
    Ok(serde_json::json!({ "screens": screens, "layout": layout }))
}

/// Chooses the screen: `primary`, `follow` (the slime goes where the pointer rests) or the number of a screen.
#[tauri::command]
fn display_save(app: tauri::AppHandle, monitor: String) -> Result<(), String> {
    let preference = slimy_display::Preference::parse(&monitor);
    display::save_preference(&soul_dir(&app)?, preference).map_err(|_| "io".to_string())
}

/// Only this rectangle (window coordinates) receives the mouse; the rest of the transparent window lets clicks through.
#[tauri::command]
fn set_hit_region(window: WebviewWindow, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let target = window.clone();
        window
            .run_on_main_thread(move || {
                use gtk::prelude::WidgetExt;
                if let Ok(gtk_window) = target.gtk_window() {
                    let region = gtk::cairo::Region::create_rectangle(
                        &gtk::cairo::RectangleInt::new(x, y, w, h),
                    );
                    gtk_window.input_shape_combine_region(Some(&region));
                }
            })
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = &window;
        *display::HIT.lock().unwrap_or_else(|e| e.into_inner()) =
            slimy_display::HitRegion { x, y, w, h };
    }
    Ok(())
}

/// Reads the pointer from the optional GNOME Shell bridge (apps/desktop/gnome-extension): Wayland hides
/// the global pointer from applications, the Shell does not.
#[cfg(target_os = "linux")]
fn bridge_call(bus_name: &str, method: &str, reply_type: &str) -> Option<gtk::glib::Variant> {
    use gtk::gio;
    let connection = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE).ok()?;
    connection
        .call_sync(
            Some(bus_name),
            "/org/slimy/Pointer",
            "org.slimy.Pointer",
            method,
            None,
            Some(gtk::glib::VariantTy::new(reply_type).ok()?),
            gio::DBusCallFlags::NO_AUTO_START,
            40,
            gio::Cancellable::NONE,
        )
        .ok()
}

#[cfg(target_os = "linux")]
fn bridge_pointer(bus_name: &str) -> Option<(f64, f64)> {
    let (x, y): (i32, i32) = bridge_call(bus_name, "GetPosition", "(ii)")?.get()?;
    Some((f64::from(x), f64::from(y)))
}

/// The focused window's class from the same bridge, or `None` when the bridge is old or missing.
#[cfg(target_os = "linux")]
fn bridge_active_app(bus_name: &str) -> Option<String> {
    let (app,): (String,) = bridge_call(bus_name, "GetActiveApp", "(s)")?.get()?;
    Some(app.chars().filter(|c| !c.is_control()).take(80).collect())
}

#[cfg(target_os = "linux")]
fn shell_pointer() -> Option<(f64, f64)> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static MISSING_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);
    if MISSING_UNTIL
        .lock()
        .ok()?
        .is_some_and(|until| Instant::now() < until)
    {
        return None;
    }
    let bus_name = std::env::var("SLIMY_POINTER_BUS").unwrap_or_else(|_| "org.gnome.Shell".into());
    let at = bridge_pointer(&bus_name);
    if at.is_none() {
        *MISSING_UNTIL.lock().ok()? = Some(Instant::now() + Duration::from_secs(10));
    }
    at
}

/// There is no Shell bridge off Linux; the dev pointer check only needs this to compile.
#[cfg(not(target_os = "linux"))]
fn shell_pointer() -> Option<(f64, f64)> {
    None
}

/// Pointer position in screen pixels: the Shell bridge when installed, else GDK (reliable on X11 only).
#[tauri::command]
async fn pointer_position(window: WebviewWindow) -> Option<(f64, f64)> {
    tauri::async_runtime::spawn_blocking(move || read_pointer_position(window))
        .await
        .ok()
        .flatten()
}

fn read_pointer_position(window: WebviewWindow) -> Option<(f64, f64)> {
    #[cfg(target_os = "linux")]
    {
        if let Some(at) = shell_pointer() {
            return Some(at);
        }
        let (tx, rx) = std::sync::mpsc::channel();
        window
            .run_on_main_thread(move || {
                use gtk::prelude::*;
                let at = gtk::gdk::Display::default()
                    .and_then(|display| display.default_seat())
                    .and_then(|seat| seat.pointer())
                    .map(|pointer| {
                        let (_, x, y) = pointer.position();
                        (f64::from(x), f64::from(y))
                    });
                let _ = tx.send(at);
            })
            .ok()?;
        rx.recv_timeout(std::time::Duration::from_millis(40))
            .ok()
            .flatten()
    }
    #[cfg(not(target_os = "linux"))]
    {
        window.cursor_position().ok().map(|p| (p.x, p.y))
    }
}

fn soul_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

/// The stored soul as JSON, or nothing when the slime is not born yet.
#[tauri::command]
fn soul_load(app: tauri::AppHandle) -> Result<Option<String>, String> {
    match store::load(&soul_dir(&app)?) {
        Ok(soul) => Ok(soul.map(|s| s.to_json())),
        Err(store::StoreError::Corrupt) => Err("corrupt".into()),
        Err(store::StoreError::Io(e)) => Err(e.to_string()),
    }
}

/// Gives birth: validates both names, stores the soul and returns it. Errors are short codes the page translates.
#[tauri::command]
fn soul_create(
    app: tauri::AppHandle,
    user_name: String,
    slime_name: String,
    language: String,
    genome_seed: Option<u64>,
) -> Result<String, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let soul = Soul::with_seed(
        &user_name,
        &slime_name,
        &language,
        now,
        genome_seed.unwrap_or(now as u64),
    )
    .map_err(|e| match e {
        SoulError::EmptyName => "empty",
        SoulError::NameTooLong => "too_long",
        SoulError::BadCharacter => "bad_character",
        SoulError::Malformed => "malformed",
    })?;
    store::save(&soul_dir(&app)?, &soul).map_err(|_| "io".to_string())?;
    Ok(soul.to_json())
}

/// How a hatchling with this seed will look, shown while it is still an egg. The same seed passed to
/// `soul_create` gives the same slime.
#[tauri::command]
fn genome_preview(seed: u64) -> serde_json::Value {
    serde_json::to_value(Genome::newborn(seed).look()).unwrap_or_default()
}

/// Opens today for an existing soul and returns how it looks, plus the night that just passed if any.
/// Null while the slime is not born yet.
#[tauri::command]
fn genome_state(app: tauri::AppHandle, today: i64) -> Result<Option<serde_json::Value>, String> {
    match store::genome_state(&soul_dir(&app)?, today) {
        Ok(state) => Ok(state.and_then(|s| serde_json::to_value(s).ok())),
        Err(store::StoreError::Corrupt) => Err("corrupt".into()),
        Err(store::StoreError::Io(e)) => Err(e.to_string()),
    }
}

/// What the page counted since its last report. The soul file bounds every value again.
#[tauri::command]
fn genome_day(
    app: tauri::AppHandle,
    today: i64,
    delta: DayLog,
) -> Result<Option<serde_json::Value>, String> {
    store::record_day(&soul_dir(&app)?, today, &delta)
        .map(|night| night.and_then(|n| serde_json::to_value(n).ok()))
        .map_err(|_| "io".to_string())
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

fn with_soul(
    app: &tauri::AppHandle,
    change: impl FnOnce(&mut Soul) -> Result<(), SoulError>,
) -> Result<(), String> {
    let dir = soul_dir(app)?;
    let mut soul = store::load(&dir)
        .map_err(|_| "io".to_string())?
        .ok_or("no_soul")?;
    change(&mut soul).map_err(|_| "invalid".to_string())?;
    store::save(&dir, &soul).map_err(|_| "io".to_string())
}

/// Sets a reminder (Unix seconds). The slime tells its human when it is due.
#[tauri::command]
fn reminder_add(
    app: tauri::AppHandle,
    at: i64,
    text: String,
    every_day: bool,
) -> Result<u32, String> {
    reminders::add(&soul_dir(&app)?, now_secs(), at, &text, every_day)
}

#[tauri::command]
fn reminder_list(app: tauri::AppHandle) -> Result<Vec<slimy_remind::Reminder>, String> {
    Ok(reminders::load(&soul_dir(&app)?).list())
}

#[tauri::command]
fn reminder_remove(app: tauri::AppHandle, id: u32) -> Result<(), String> {
    if reminders::remove(&soul_dir(&app)?, id)? {
        Ok(())
    } else {
        Err("reminder_unknown".into())
    }
}

/// The reminders that are due now (each is returned once).
#[tauri::command]
fn reminders_due(app: tauri::AppHandle) -> Result<Vec<slimy_remind::Reminder>, String> {
    reminders::due(&soul_dir(&app)?, now_secs())
}

/// What the slime remembers about its human, to show and edit.
#[tauri::command]
fn memory_list(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let soul = store::load(&soul_dir(&app)?)
        .map_err(|_| "io".to_string())?
        .ok_or("no_soul")?;
    Ok(serde_json::json!(soul
        .facts
        .iter()
        .rev()
        .map(|f| serde_json::json!({ "key": f.key, "value": f.value, "learned_at": f.learned_at }))
        .collect::<Vec<_>>()))
}

/// Forgets one remembered fact.
#[tauri::command]
fn memory_forget(app: tauri::AppHandle, key: String) -> Result<(), String> {
    let mut found = false;
    with_soul(&app, |soul| {
        found = soul.forget(&key);
        Ok(())
    })?;
    if found {
        Ok(())
    } else {
        Err("memory_unknown".into())
    }
}

/// Remembers something the user told the slime.
#[tauri::command]
fn soul_learn(app: tauri::AppHandle, key: String, value: String) -> Result<(), String> {
    with_soul(&app, |soul| soul.learn(&key, &value, now_secs()))
}

/// Stores how the slime feels (values are clamped by the soul).
#[tauri::command]
fn soul_feelings(
    app: tauri::AppHandle,
    affection: f32,
    trust: f32,
    resentment: f32,
) -> Result<(), String> {
    with_soul(&app, |soul| {
        soul.relation = slimy_core::Relation {
            affection,
            trust,
            resentment,
        }
        .clamped();
        Ok(())
    })
}

#[derive(serde::Deserialize)]
struct Turn {
    role: String,
    text: String,
    #[serde(default)]
    image: Option<String>,
}

#[derive(Serialize)]
struct SpokenReply {
    say: String,
    expression: Option<String>,
    valence: f32,
    arousal: f32,
    tool: Option<slimy_brain::ToolRequest>,
}

fn brain_error(e: slimy_brain::BrainError) -> String {
    match e {
        slimy_brain::BrainError::Provider(message) => format!("provider:{message}"),
        slimy_brain::BrainError::Http(status) => format!("provider:HTTP {status}"),
        other => other.code().to_owned(),
    }
}

/// Providers the brain can talk to, for the settings panel.
#[tauri::command]
fn brain_providers() -> serde_json::Value {
    serde_json::to_value(slimy_brain::PROVIDERS).unwrap_or_default()
}

/// The current brain settings and whether a key is stored for the chosen provider (never the key).
#[tauri::command]
async fn brain_config(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let config = brain::load_config(&soul_dir(&app)?);
    let has_key = secrets::load(&config.provider).await.is_some();
    Ok(serde_json::json!({ "config": config, "has_key": has_key }))
}

#[tauri::command]
async fn brain_has_key(provider: String) -> bool {
    secrets::load(&provider).await.is_some()
}

#[tauri::command]
fn brain_save(app: tauri::AppHandle, config: String) -> Result<(), String> {
    let config = slimy_brain::BrainConfig::from_json(&config).map_err(brain_error)?;
    brain::save_config(&soul_dir(&app)?, &config).map_err(|_| "io".to_string())
}

#[tauri::command]
async fn brain_set_key(provider: String, key: String) -> Result<(), String> {
    if slimy_brain::PROVIDERS.iter().all(|p| p.id != provider) {
        return Err("unknown_provider".into());
    }
    secrets::save(&provider, &key).await
}

#[tauri::command]
async fn brain_forget_key(provider: String) -> Result<(), String> {
    secrets::remove(&provider).await
}

/// One turn of conversation. The reply is already tidied: short text, a known expression, clamped feelings.
#[tauri::command]
async fn brain_chat(
    app: tauri::AppHandle,
    history: Vec<Turn>,
    sulking: bool,
) -> Result<SpokenReply, String> {
    let dir = soul_dir(&app)?;
    let soul = store::load(&dir)
        .map_err(|_| "io".to_string())?
        .ok_or("no_soul")?;
    let config = brain::load_config(&dir);
    let turns: Vec<slimy_brain::Message> = history
        .into_iter()
        .map(|t| slimy_brain::Message {
            role: if t.role == "assistant" {
                slimy_brain::Role::Assistant
            } else {
                slimy_brain::Role::User
            },
            text: t.text,
            image: t.image,
        })
        .collect();
    let offers = tool_offers(&app).await;
    let mut reply = brain::chat(&config, &soul, sulking, &turns, &offers).await;
    // A model that mistakes the list of powers for function tools is refused by some providers: answer without the list.
    if !offers.is_empty()
        && reply
            .as_ref()
            .err()
            .is_some_and(slimy_brain::BrainError::is_tool_confusion)
    {
        reply = brain::chat(&config, &soul, sulking, &turns, &[]).await;
    }
    // The chosen provider is down or limited: try the fallback once, with its own key.
    if reply
        .as_ref()
        .err()
        .is_some_and(slimy_brain::BrainError::is_retryable)
    {
        if let Some(second) = config.fallback_config() {
            reply = brain::chat(&second, &soul, sulking, &turns, &offers).await;
        }
    }
    let reply = reply.map_err(brain_error)?;
    Ok(SpokenReply {
        say: reply.say,
        expression: reply.expression,
        valence: reply.valence,
        arousal: reply.arousal,
        tool: reply.tool,
    })
}

/// The text of one of the user's skills.
#[tauri::command]
fn skill_read(app: tauri::AppHandle, name: String) -> Result<String, String> {
    skills::load(&soul_dir(&app)?)
        .into_iter()
        .find(|s| s.name == name)
        .map(|s| s.body)
        .ok_or_else(|| "skill_unknown".to_string())
}

/// The webview has no visible console, so page errors are forwarded to the terminal.
/// Powers, which are granted now, and the latest journal entries.
#[tauri::command]
fn perms_state(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    Ok(perms::view(&soul_dir(&app)?, now_secs().max(0) as u64))
}

/// Grants one power. Screen, clipboard and pointer need `seconds`, capped at one hour.
#[tauri::command]
fn perms_grant(app: tauri::AppHandle, power: String, seconds: Option<u64>) -> Result<(), String> {
    let power = perms::power(&power)?;
    let seconds = seconds.map(|s| s.clamp(1, 3600));
    let now = now_secs().max(0) as u64;
    perms::change(&soul_dir(&app)?, |r| r.grant(power, now, seconds))?
        .map_err(|_| "duration".to_string())
}

#[tauri::command]
fn perms_revoke(app: tauri::AppHandle, power: Option<String>) -> Result<(), String> {
    let power = power.map(|p| perms::power(&p)).transpose()?;
    if power.is_none_or(|p| p == slimy_perms::Power::Pointer) {
        tauri::async_runtime::block_on(remote::close());
    }
    perms::change(&soul_dir(&app)?, |r| match power {
        Some(p) => {
            r.revoke(p);
        }
        None => r.revoke_all(),
    })
}

/// Runs `work` for a declared server only when the `tool` power is granted; the decision is journaled.
fn with_tool<T>(
    app: &tauri::AppHandle,
    server: &str,
    note: String,
    work: impl FnOnce(slimy_mcp::Server) -> Result<T, String>,
) -> Result<T, String> {
    gated(
        &soul_dir(app)?,
        server,
        now_secs().max(0) as u64,
        &note,
        work,
    )
}

fn gated<T>(
    dir: &std::path::Path,
    server: &str,
    now: u64,
    note: &str,
    work: impl FnOnce(slimy_mcp::Server) -> Result<T, String>,
) -> Result<T, String> {
    perms::allow(dir, slimy_perms::Power::Tool, now, note)?;
    work(mcp::find(dir, server)?)
}

/// How long the tool list is reused, so a chat does not start every server each time.
const OFFERS_TTL: std::time::Duration = std::time::Duration::from_secs(300);
static OFFERS: std::sync::Mutex<Option<(std::time::Instant, Vec<slimy_brain::ToolOffer>)>> =
    std::sync::Mutex::new(None);

/// The tools the brain may ask for: none unless the `tool` power is granted right now. A server that
/// fails is skipped, so a broken tool never breaks the conversation.
async fn tool_offers(app: &tauri::AppHandle) -> Vec<slimy_brain::ToolOffer> {
    let Ok(dir) = soul_dir(app) else {
        return Vec::new();
    };
    let now = now_secs().max(0) as u64;
    let mut offers = Vec::new();
    // Built in: the slime's own reading powers, each offered only while granted.
    if perms::load(&dir).is_granted(slimy_perms::Power::Clipboard, now) {
        offers.push(slimy_brain::ToolOffer {
            server: BUILTIN.into(),
            name: "clipboard".into(),
            description: "Reads the text the user copied last".into(),
        });
    }
    if perms::load(&dir).is_granted(slimy_perms::Power::ActiveApp, now) {
        offers.push(slimy_brain::ToolOffer {
            server: BUILTIN.into(),
            name: "active_app".into(),
            description: "Names the application the user is looking at right now".into(),
        });
    }
    if perms::load(&dir).is_granted(slimy_perms::Power::Screen, now) {
        offers.push(slimy_brain::ToolOffer { server: BUILTIN.into(), name: "screen".into(), description: "Looks at a picture of the user's screen (sent to the chosen AI provider, so only ask when it really helps)".into() });
    }
    if perms::load(&dir).is_granted(slimy_perms::Power::Pointer, now) {
        offers.push(slimy_brain::ToolOffer { server: BUILTIN.into(), name: "pointer".into(), description: "Nudges the real mouse pointer; arguments {\"dx\": pixels right, \"dy\": pixels down}, at most 300 each".into() });
    }
    if perms::load(&dir).is_granted(slimy_perms::Power::Files, now) && files::root(&dir).is_some() {
        offers.push(slimy_brain::ToolOffer { server: BUILTIN.into(), name: "files".into(), description: "Reads a file or lists a folder of the shared folder; arguments {\"path\": \"relative/path\"}, empty path for the top".into() });
    }
    // Always offered: they only write the slime's own notes, and each use is confirmed with a yes or no.
    offers.push(slimy_brain::ToolOffer { server: BUILTIN.into(), name: "remember".into(), description: "Keeps one fact about the user for later; arguments {\"key\": \"short name\", \"value\": \"the fact\"}".into() });
    offers.push(slimy_brain::ToolOffer { server: BUILTIN.into(), name: "remind".into(), description: "Reminds the user later; arguments {\"text\": \"what to say\", \"minutes\": 20} or {\"text\": \"...\", \"at\": \"18:30\", \"every_day\": false}".into() });
    let known = skills::load(&dir);
    if !known.is_empty() {
        let list = known
            .iter()
            .map(|s| format!("{} ({})", s.name, s.description))
            .collect::<Vec<_>>()
            .join("; ");
        offers.push(slimy_brain::ToolOffer { server: BUILTIN.into(), name: "skill".into(), description: format!("Reads one of the user's own skills, written instructions to follow; arguments {{\"name\": \"skill name\"}}; available: {list}") });
    }
    offers.extend(mcp_offers(app, &dir, now).await);
    offers
}

/// The name under which the slime's own powers are offered next to the MCP servers.
const BUILTIN: &str = "slimy";

async fn mcp_offers(
    app: &tauri::AppHandle,
    dir: &std::path::Path,
    now: u64,
) -> Vec<slimy_brain::ToolOffer> {
    if !perms::load(dir).is_granted(slimy_perms::Power::Tool, now) {
        return Vec::new();
    }
    if let Some((at, list)) = OFFERS.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        if at.elapsed() < OFFERS_TTL {
            return list.clone();
        }
    }
    let app = app.clone();
    let list = tauri::async_runtime::spawn_blocking(move || {
        let names = mcp::load(&soul_dir(&app).unwrap_or_default()).unwrap_or_default();
        names
            .iter()
            .flat_map(|s| {
                with_tool(&app, &s.name, format!("{}: list", s.name), |s| {
                    mcp::tools(&s)
                })
                .unwrap_or_default()
                .into_iter()
                .map(|t| slimy_brain::ToolOffer {
                    server: s.name.clone(),
                    name: t.name,
                    description: t.description,
                })
                .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default();
    *OFFERS.lock().unwrap_or_else(|e| e.into_inner()) =
        Some((std::time::Instant::now(), list.clone()));
    list
}

/// The clipboard text, only while the `clipboard` power is granted. The access is journaled with no content.
#[tauri::command]
async fn clipboard_read(app: tauri::AppHandle) -> Result<String, String> {
    perms::allow(
        &soul_dir(&app)?,
        slimy_perms::Power::Clipboard,
        now_secs().max(0) as u64,
        "read",
    )?;
    tauri::async_runtime::spawn_blocking(clipboard::read)
        .await
        .ok()
        .flatten()
        .ok_or_else(|| "clipboard_empty".to_string())
}

/// The class of the focused window (for example `firefox`), only while the `active_app` power is granted.
#[tauri::command]
async fn active_app(app: tauri::AppHandle) -> Result<String, String> {
    perms::allow(
        &soul_dir(&app)?,
        slimy_perms::Power::ActiveApp,
        now_secs().max(0) as u64,
        "read",
    )?;
    #[cfg(target_os = "linux")]
    {
        let bus = std::env::var("SLIMY_POINTER_BUS").unwrap_or_else(|_| "org.gnome.Shell".into());
        tauri::async_runtime::spawn_blocking(move || bridge_active_app(&bus))
            .await
            .ok()
            .flatten()
            .ok_or_else(|| "active_app_missing".to_string())
    }
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        // The application name only, never the window title.
        return tauri::async_runtime::spawn_blocking(|| {
            active_win_pos_rs::get_active_window().ok().map(|w| {
                w.app_name
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(80)
                    .collect::<String>()
            })
        })
        .await
        .ok()
        .flatten()
        .ok_or_else(|| "active_app_missing".to_string());
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    Err("active_app_missing".to_string())
}

/// Moves the real pointer by (dx, dy) pixels (at most 300 each), only while the `pointer` power is granted.
/// The system asks the user once more in its own dialog the first time.
#[tauri::command]
async fn pointer_nudge(app: tauri::AppHandle, dx: i32, dy: i32) -> Result<(), String> {
    let dir = soul_dir(&app)?;
    if let Err(e) = perms::allow(
        &dir,
        slimy_perms::Power::Pointer,
        now_secs().max(0) as u64,
        &format!("{dx},{dy}"),
    ) {
        remote::close().await;
        return Err(e);
    }
    remote::nudge(dx, dy).await
}

/// One picture of the screen as base64 JPEG, only while the `screen` power is granted. The picture is not stored.
#[tauri::command]
async fn screen_capture(app: tauri::AppHandle) -> Result<String, String> {
    perms::allow(
        &soul_dir(&app)?,
        slimy_perms::Power::Screen,
        now_secs().max(0) as u64,
        "capture",
    )?;
    screen::capture().await
}

static LISTEN_CANCEL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SPEAK_CANCEL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn voice_of(app: &tauri::AppHandle, language: &str) -> Result<slimy_voice::Voice, String> {
    let dir = soul_dir(app)?;
    let soul = store::load(&dir)
        .map_err(|_| "io".to_string())?
        .ok_or("no_soul")?;
    let traits = slimy_voice::Traits {
        affection: soul.tone.affection,
        curiosity: soul.tone.curiosity,
        energy: soul.tone.energy,
        size: soul.genome.get(slimy_core::Gene::Size),
    };
    Ok(slimy_voice::voice_for(
        language,
        traits,
        voice::load_config(&dir).style,
    ))
}

/// Which voice models are installed, the settings, and the voice this slime has.
#[tauri::command]
fn voice_status(app: tauri::AppHandle, language: String) -> Result<serde_json::Value, String> {
    let dir = soul_dir(&app)?;
    let voice = voice_of(&app, &language)
        .ok()
        .map(|v| serde_json::json!({ "name": v.name, "speed": v.speed, "pitch": v.pitch }));
    Ok(serde_json::json!({
        "installed": voice::installed(&dir),
        "config": voice::load_config(&dir),
        "megabytes": { "speech": voice::SPEECH.megabytes, "hearing": voice::HEARING.megabytes, "hearing_small": voice::HEARING_SMALL.megabytes },
        "voice": voice,
        "french": voice::FRENCH,
    }))
}

/// Downloads one voice model from the official release (126 or 198 MB), reporting progress through `voice-install` events.
#[tauri::command]
async fn voice_install(app: tauri::AppHandle, which: String) -> Result<(), String> {
    use tauri::Emitter;
    let what = match which.as_str() {
        "speech" => &voice::SPEECH,
        "hearing" => &voice::HEARING,
        "hearing_small" => &voice::HEARING_SMALL,
        _ => return Err("voice_install".into()),
    };
    let dir = soul_dir(&app)?;
    let events = app.clone();
    let name = what.name;
    let mut last = 0u64;
    voice::install(&dir, what, move |done, total| {
        // About one event per percent, so the page is not flooded.
        if done == total || done / (total / 100 + 1) != last {
            last = done / (total / 100 + 1);
            let _ = events.emit(
                "voice-install",
                serde_json::json!({ "which": name, "done": done, "total": total }),
            );
        }
    })
    .await
}

#[tauri::command]
fn voice_config_save(
    app: tauri::AppHandle,
    replies: bool,
    style: String,
    hearing: Option<String>,
) -> Result<(), String> {
    let style = match style.as_str() {
        "auto" => slimy_voice::Style::Auto,
        "kawaii" => slimy_voice::Style::Kawaii,
        "calm" => slimy_voice::Style::Calm,
        _ => return Err("voice_style".into()),
    };
    voice::save_config(
        &soul_dir(&app)?,
        &voice::Config {
            replies,
            style,
            hearing: slimy_voice::Hearing::parse(hearing.as_deref().unwrap_or("base")),
        },
    )
    .map_err(|_| "io".to_string())
}

/// Listens through the microphone until the person has spoken and paused, and returns what was said. Needs the
/// `microphone` power; `voice-level` events carry the loudness for the page to show.
#[tauri::command]
async fn voice_listen(app: tauri::AppHandle, language: String) -> Result<String, String> {
    use std::sync::atomic::Ordering;
    use tauri::Emitter;
    let dir = soul_dir(&app)?;
    perms::allow(
        &dir,
        slimy_perms::Power::Microphone,
        now_secs().max(0) as u64,
        "listen",
    )?;
    if !voice::installed(&dir).hearing {
        return Err("voice_missing".into());
    }
    LISTEN_CANCEL.store(false, Ordering::Relaxed);
    let events = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let samples = voice::record(&LISTEN_CANCEL, |level| {
            let _ = events.emit(
                "voice-level",
                serde_json::json!({ "kind": "mic", "level": level }),
            );
        })?;
        let _ = events.emit(
            "voice-level",
            serde_json::json!({ "kind": "mic", "level": 0.0 }),
        );
        if samples.is_empty() {
            return Err("voice_nobody".to_string());
        }
        let text = voice::transcribe(&dir, &samples, &language)?;
        // Whisper marks silence and noise with bracketed tags: those are not words.
        let cleaned: String = text
            .split_whitespace()
            .filter(|w| !(w.starts_with('[') || w.starts_with('(') && w.ends_with(')')))
            .collect::<Vec<_>>()
            .join(" ");
        if cleaned.trim().is_empty() {
            Err("voice_nobody".to_string())
        } else {
            Ok(cleaned)
        }
    })
    .await
    .map_err(|_| "voice_failed".to_string())?
}

#[tauri::command]
fn voice_stop_listening() {
    LISTEN_CANCEL.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Says `text` aloud with this slime's voice, coloured by how it feels (the face, valence and arousal of the answer), moving
/// its mouth through `voice-level` events. The first sentence starts as soon as it is ready. Returns when it has finished
/// or been stopped. Nothing is recorded and nothing leaves the computer.
#[tauri::command]
async fn voice_speak(
    app: tauri::AppHandle,
    text: String,
    language: String,
    expression: Option<String>,
    valence: Option<f32>,
    arousal: Option<f32>,
    pace: Option<f32>,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    use tauri::Emitter;
    let dir = soul_dir(&app)?;
    if !voice::installed(&dir).speech {
        return Err("voice_missing".into());
    }
    let spoken = slimy_voice::speakable_in(&text, &language);
    if spoken.is_empty() {
        return Ok(());
    }
    let mut voice = voice_of(&app, &language)?.with_mood(
        expression.as_deref(),
        valence.unwrap_or(0.0),
        arousal.unwrap_or(0.3),
    );
    // The learned pace is one of three fixed multipliers; bounded again here whatever the page sends.
    voice.speed = (voice.speed
        * pace
            .filter(|p| p.is_finite())
            .unwrap_or(1.0)
            .clamp(0.9, 1.1))
    .clamp(0.8, 1.3);
    SPEAK_CANCEL.store(false, Ordering::Relaxed);
    let events = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        voice::speak(
            &dir,
            &spoken,
            &language,
            &voice,
            &SPEAK_CANCEL,
            |level| {
                let _ = events.emit(
                    "voice-level",
                    serde_json::json!({ "kind": "voice", "level": level }),
                );
            },
            |_| {},
        )
    })
    .await
    .map_err(|_| "voice_failed".to_string())?
}

fn learn_seed(app: &tauri::AppHandle) -> (std::path::PathBuf, u64) {
    let dir = soul_dir(app).unwrap_or_default();
    let seed = store::load(&dir)
        .ok()
        .flatten()
        .map_or(7, |s| s.born_at as u64);
    (dir, seed)
}

/// The option the slime should use now for one of its small choices, learned from how its human reacted before:
/// `{ arm, value }`. `hour` is the local hour (0..24).
#[tauri::command]
fn learn_choose(
    app: tauri::AppHandle,
    decision: String,
    hour: u32,
) -> Result<serde_json::Value, String> {
    let (dir, seed) = learn_seed(&app);
    let (arm, value) = learn::choose(&dir, seed, &decision, hour)?;
    Ok(serde_json::json!({ "arm": arm, "value": value }))
}

/// How the human reacted to option `arm`, from -1 (badly) to 1 (very well).
#[tauri::command]
fn learn_reward(
    app: tauri::AppHandle,
    decision: String,
    hour: u32,
    arm: usize,
    reward: f32,
) -> Result<(), String> {
    let (dir, seed) = learn_seed(&app);
    learn::reward(&dir, seed, &decision, hour, arm, reward)
}

/// What the slime has learned so far, to show the user.
#[tauri::command]
fn learn_report(app: tauri::AppHandle) -> Result<Vec<slimy_learn::Belief>, String> {
    let (dir, seed) = learn_seed(&app);
    learn::report(&dir, seed)
}

/// Forgets everything learned.
#[tauri::command]
fn learn_reset(app: tauri::AppHandle) -> Result<(), String> {
    let (dir, seed) = learn_seed(&app);
    learn::reset(&dir, seed)
}

/// Loads the voice models into memory ahead of time, so the first word is not slow. Harmless when nothing is installed.
#[tauri::command]
async fn voice_warm(app: tauri::AppHandle, language: String) -> Result<(), String> {
    let dir = soul_dir(&app)?;
    let installed = voice::installed(&dir);
    let voice = voice_of(&app, &language).ok();
    tauri::async_runtime::spawn_blocking(move || {
        if let (true, Some(voice)) = (installed.speech, voice) {
            let _ = voice::synthesize(&dir, "Salut.", &language, &voice);
        }
        if installed.hearing {
            let _ = voice::transcribe(&dir, &vec![0.0f32; 8000], &language);
        }
    })
    .await
    .map_err(|_| "voice_failed".to_string())
}

#[tauri::command]
fn voice_stop_speaking() {
    SPEAK_CANCEL.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// A file or folder of the declared folder, only while the `files` power is granted.
#[tauri::command]
async fn files_read(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let dir = soul_dir(&app)?;
    let root = files::root(&dir).ok_or_else(|| "files_root".to_string())?;
    perms::allow(
        &dir,
        slimy_perms::Power::Files,
        now_secs().max(0) as u64,
        &path.chars().take(60).collect::<String>(),
    )?;
    tauri::async_runtime::spawn_blocking(move || files::read(&root, &path))
        .await
        .map_err(|_| "io".to_string())?
}

/// Names of the servers declared in `mcp.json` (never their commands).
#[tauri::command]
fn mcp_servers(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    Ok(mcp::load(&soul_dir(&app)?)
        .map_err(|_| "mcp_config".to_string())?
        .into_iter()
        .map(|s| s.name)
        .collect())
}

#[tauri::command]
async fn mcp_tools(app: tauri::AppHandle, server: String) -> Result<Vec<slimy_mcp::Tool>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        with_tool(&app, &server.clone(), format!("{server}: list"), |s| {
            mcp::tools(&s)
        })
    })
    .await
    .map_err(|_| "mcp_io".to_string())?
}

#[tauri::command]
async fn mcp_call(
    app: tauri::AppHandle,
    server: String,
    tool: String,
    arguments: serde_json::Value,
) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let note = format!("{server}: {tool}");
        with_tool(&app, &server.clone(), note, |s| {
            mcp::call(&s, &tool, arguments)
        })
    })
    .await
    .map_err(|_| "mcp_io".to_string())?
    .map(|(text, failed)| serde_json::json!({ "text": text, "failed": failed }))
}

#[tauri::command]
fn log_error(message: String) {
    eprintln!("[slimy web] {message}");
}

#[tauri::command]
fn quit(app: tauri::AppHandle) {
    app.exit(0);
}

/// The window covers the whole usable screen; only the slime's hit region receives the mouse.
#[tauri::command]
fn cover_world(window: WebviewWindow, area: World) -> Result<(), String> {
    window
        .set_size(PhysicalSize::new(
            (area.right - area.left) as u32,
            (area.bottom - area.top) as u32,
        ))
        .map_err(|e| e.to_string())?;
    window
        .set_position(PhysicalPosition::new(area.left as i32, area.top as i32))
        .map_err(|e| e.to_string())
}

/// `slimy-desktop set-key <provider>` reads the key on stdin and stores it in the keyring.
/// `slimy-desktop brain-test` sends one greeting through the configured brain and prints the answer.
fn command_line() -> bool {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("set-key") => {
            let provider = args.get(2).cloned().unwrap_or_default();
            let mut key = String::new();
            let _ = std::io::stdin().read_line(&mut key);
            let known = slimy_brain::PROVIDERS.iter().any(|p| p.id == provider);
            let result = if known && !key.trim().is_empty() {
                tauri::async_runtime::block_on(secrets::save(&provider, &key))
            } else {
                Err("unknown provider or empty key".into())
            };
            match result {
                Ok(()) => println!("key stored for {provider}"),
                Err(e) => {
                    eprintln!("could not store the key: {e}");
                    std::process::exit(1)
                }
            }
            true
        }
        #[cfg(all(debug_assertions, target_os = "linux"))]
        Some("drive") => {
            // Dev only: `slimy-desktop drive COMMANDS_FILE ANSWERS_FILE`, see drive.rs.
            let args: Vec<String> = std::env::args().skip(2).collect();
            if let [commands, answers] = &args[..] {
                if let Err(e) = tauri::async_runtime::block_on(drive::run(commands, answers)) {
                    eprintln!("drive failed: {e}");
                    std::process::exit(1)
                }
            }
            true
        }
        Some("listen-test") => {
            // Dev check of the microphone path: records until the person has spoken and paused, then says what it heard.
            let data = std::env::var_os("XDG_DATA_HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_default()
                .join("dev.slimy.desktop");
            let language = std::env::args().nth(2).unwrap_or("fr".into());
            let stop = std::sync::atomic::AtomicBool::new(false);
            let mut loudest = 0f32;
            let mut trace: Vec<f32> = Vec::new();
            let started = std::time::Instant::now();
            let outcome = voice::record(&stop, |l| {
                loudest = loudest.max(l);
                trace.push(l);
            });
            if std::env::var_os("SLIMY_DEBUG_RAW").is_some() {
                println!(
                    "levels: {:?}",
                    trace
                        .iter()
                        .map(|l| (l * 1000.0).round() / 1000.0)
                        .take(60)
                        .collect::<Vec<_>>()
                );
            }
            match outcome {
                Ok(samples) => {
                    println!(
                        "recorded {:.1} s in {:.1} s, loudest {loudest:.3}",
                        samples.len() as f32 / 16000.0,
                        started.elapsed().as_secs_f32()
                    );
                    if samples.is_empty() {
                        println!("nobody spoke");
                    } else {
                        println!("heard: {:?}", voice::transcribe(&data, &samples, &language));
                    }
                }
                Err(e) => {
                    eprintln!("listen failed: {e}");
                    std::process::exit(1)
                }
            }
            true
        }
        Some("voice-test") => {
            // Dev check of the voice engine: says a sentence, then listens to the recording to see it was understood.
            let data = std::env::var_os("XDG_DATA_HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_default()
                .join("dev.slimy.desktop");
            let args: Vec<String> = std::env::args().skip(2).collect();
            let (language, speaker, text) = (
                args.first().cloned().unwrap_or("fr".into()),
                args.get(1).and_then(|s| s.parse().ok()).unwrap_or(30),
                args.get(2).cloned().unwrap_or(
                    "Bonjour, je suis un petit slime et je suis content de te voir.".into(),
                ),
            );
            println!("installed: {:?}", voice::installed(&data));
            if std::env::var_os("SLIMY_STREAM").is_some() {
                // Says the text the way the app does (sentence by sentence) and reports when the first sound began.
                let mood = std::env::var("SLIMY_MOOD").ok();
                let voice = slimy_voice::Voice {
                    speaker: args.get(1).and_then(|s| s.parse().ok()).unwrap_or(30),
                    name: "test",
                    speed: 1.0,
                    pitch: 1.05,
                }
                .with_mood(mood.as_deref(), 0.5, 0.5);
                let loading = std::time::Instant::now();
                let _ = voice::synthesize(&data, "Salut.", &language, &voice);
                println!(
                    "model loaded in {:.2} s (the app does this in the background after start)",
                    loading.elapsed().as_secs_f32()
                );
                let stop = std::sync::atomic::AtomicBool::new(false);
                let started = std::time::Instant::now();
                let mut first = None;
                let result = voice::speak(
                    &data,
                    &slimy_voice::speakable_in(&text, &language),
                    &language,
                    &voice,
                    &stop,
                    |_| {},
                    |d| first = Some(d),
                );
                println!(
                    "streamed: {result:?}, first sound after {:.2} s, finished after {:.2} s",
                    first.map_or(-1.0, |d| d.as_secs_f32()),
                    started.elapsed().as_secs_f32()
                );
                return true;
            }
            let voice = slimy_voice::Voice {
                speaker,
                name: "test",
                speed: 1.0,
                pitch: 1.0,
            };
            let started = std::time::Instant::now();
            match voice::synthesize(&data, &slimy_voice::speakable(&text), &language, &voice) {
                Ok((samples, rate)) => {
                    println!(
                        "spoken: {} samples at {rate} Hz ({:.1} s of audio) in {:.1} s",
                        samples.len(),
                        samples.len() as f32 / rate as f32,
                        started.elapsed().as_secs_f32()
                    );
                    let heard_at = std::time::Instant::now();
                    let mut heard = voice::to_listen_rate(&samples, rate);
                    let tail = |s: &[f32]| {
                        (s.iter().map(|v| v * v).sum::<f32>() / s.len().max(1) as f32).sqrt()
                    };
                    println!(
                        "loudness of the first and last 100 ms: {:.3} / {:.3}",
                        tail(&samples[..2400.min(samples.len())]),
                        tail(&samples[samples.len().saturating_sub(2400)..])
                    );
                    if std::env::var_os("SLIMY_PAD").is_some() {
                        let silence = vec![0.0f32; 8000];
                        heard = [silence.clone(), heard, silence].concat();
                    }
                    if let Some(path) = std::env::var_os("SLIMY_WAV") {
                        // 16-bit mono WAV of what was said, for playing it later from another process.
                        let pcm: Vec<u8> = samples
                            .iter()
                            .flat_map(|s| ((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
                            .collect();
                        let mut wav = Vec::new();
                        wav.extend_from_slice(b"RIFF");
                        wav.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
                        wav.extend_from_slice(b"WAVEfmt ");
                        wav.extend_from_slice(&16u32.to_le_bytes());
                        wav.extend_from_slice(&1u16.to_le_bytes());
                        wav.extend_from_slice(&1u16.to_le_bytes());
                        wav.extend_from_slice(&rate.to_le_bytes());
                        wav.extend_from_slice(&(rate * 2).to_le_bytes());
                        wav.extend_from_slice(&2u16.to_le_bytes());
                        wav.extend_from_slice(&16u16.to_le_bytes());
                        wav.extend_from_slice(b"data");
                        wav.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
                        wav.extend_from_slice(&pcm);
                        println!("wav written: {:?}", std::fs::write(&path, wav));
                    }
                    if std::env::var_os("SLIMY_PLAY").is_some() {
                        let silent = std::sync::atomic::AtomicBool::new(false);
                        let mut peak = 0f32;
                        let r = voice::play(samples.clone(), rate, &silent, |l| peak = peak.max(l));
                        println!("played: {r:?}, loudest moment {peak:.3}");
                    }
                    let back = voice::transcribe(&data, &heard, &language);
                    println!(
                        "heard back in {:.1} s: {back:?}",
                        heard_at.elapsed().as_secs_f32()
                    );
                }
                Err(e) => {
                    eprintln!("voice failed: {e}");
                    std::process::exit(1)
                }
            }
            true
        }
        Some("clipboard-test") => {
            // Dev check of the clipboard path: prints what the app would read (the length and the first characters only).
            match clipboard::read() {
                Some(text) => println!(
                    "clipboard: {} characters, starts with {:?}",
                    text.chars().count(),
                    text.chars().take(24).collect::<String>()
                ),
                None => println!("clipboard: nothing readable"),
            }
            true
        }
        Some("screen-test") => {
            // Dev check of the capture path: reports the size of the prepared picture, never the picture itself.
            match tauri::async_runtime::block_on(screen::capture()) {
                Ok(text) => {
                    use base64::Engine;
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(&text)
                        .unwrap_or_default();
                    let stats = xcap::image::load_from_memory(&bytes).map(|i| {
                        let gray = i.to_luma8();
                        let n = gray.len() as f64;
                        let mean = gray.iter().map(|v| f64::from(*v)).sum::<f64>() / n;
                        let sd = (gray
                            .iter()
                            .map(|v| (f64::from(*v) - mean).powi(2))
                            .sum::<f64>()
                            / n)
                            .sqrt();
                        (i.width(), i.height(), mean, sd)
                    });
                    println!(
                        "captured: {} bytes of base64 JPEG, size/mean/stddev {stats:?}",
                        text.len()
                    );
                }
                Err(e) => {
                    eprintln!("capture failed: {e}");
                    std::process::exit(1)
                }
            }
            true
        }
        Some("pointer-test") => {
            // Dev check of the RemoteDesktop path: nudges the pointer 80 px right and back. The system asks for consent first.
            let before = shell_pointer();
            let outcome = tauri::async_runtime::block_on(async {
                remote::nudge(80, 0).await?;
                let mid = shell_pointer();
                remote::nudge(-80, 0).await?;
                Ok::<_, String>(mid)
            });
            println!(
                "before {before:?}, outcome {outcome:?}, after {:?}",
                shell_pointer()
            );
            true
        }
        Some("bridge-test") => {
            // Dev check of the GNOME bridge: prints the focused window class and the pointer position.
            #[cfg(target_os = "linux")]
            {
                let bus =
                    std::env::var("SLIMY_POINTER_BUS").unwrap_or_else(|_| "org.gnome.Shell".into());
                println!(
                    "active app: {:?}, pointer: {:?}",
                    bridge_active_app(&bus),
                    bridge_pointer(&bus)
                );
            }
            true
        }
        Some("brain-test") => {
            let dir = std::env::var_os("XDG_DATA_HOME")
                .map(std::path::PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME")
                        .map(|h| std::path::PathBuf::from(h).join(".local/share"))
                })
                .unwrap_or_default()
                .join("dev.slimy.desktop");
            let soul = store::load(&dir)
                .ok()
                .flatten()
                .unwrap_or_else(|| Soul::new("toi", "Slimy", "fr", 0).expect("valid names"));
            let config = brain::load_config(&dir);
            // `brain-test [message]`: one real turn, with the built-in powers offered so the model's way of asking can be seen.
            let text = std::env::args()
                .nth(2)
                .unwrap_or_else(|| "Salut ! Tu es là ?".into());
            let hello = [slimy_brain::Message {
                role: slimy_brain::Role::User,
                text,
                image: None,
            }];
            let offer = |name: &str, description: &str| slimy_brain::ToolOffer {
                server: BUILTIN.into(),
                name: name.into(),
                description: description.into(),
            };
            let offers = [
                offer("files", "Reads a file or lists a folder of the shared folder; arguments {\"path\": \"relative/path\"}, empty path for the top"),
                offer("clipboard", "Reads the text the user copied last"),
                offer("active_app", "Names the application the user is looking at right now"),
            ];
            match tauri::async_runtime::block_on(brain::chat(
                &config, &soul, false, &hello, &offers,
            )) {
                Ok(r) => println!(
                    "{} via {} ({}): {} [{}] ask={:?}",
                    soul.slime_name,
                    config.provider,
                    config.model,
                    r.say,
                    r.expression.unwrap_or_default(),
                    r.tool
                ),
                Err(e) => {
                    eprintln!("brain error: {}", brain_error(e));
                    std::process::exit(1)
                }
            }
            true
        }
        _ => false,
    }
}

fn main() {
    if command_line() {
        return;
    }
    // Wayland refuses client side window positioning, so the roaming window needs X11 (XWayland).
    #[cfg(target_os = "linux")]
    if std::env::var_os("GDK_BACKEND").is_none() {
        std::env::set_var("GDK_BACKEND", "x11");
    }
    tauri::Builder::default()
        .setup(|app| {
            if let Ok(dir) = soul_dir(app.handle()) {
                display::watch(app.handle().clone(), dir);
            }
            // The slime follows the user across virtual desktops instead of staying on the one it was born on.
            if let Some(window) = tauri::Manager::webview_windows(app.handle())
                .into_values()
                .next()
            {
                let _ = window.set_visible_on_all_workspaces(true);
            }
            #[cfg(not(target_os = "linux"))]
            display::click_through(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            world,
            display_status,
            display_save,
            cover_world,
            set_hit_region,
            pointer_position,
            soul_load,
            soul_create,
            soul_learn,
            memory_list,
            memory_forget,
            reminder_add,
            reminder_list,
            reminder_remove,
            reminders_due,
            slimefile::slime_export,
            slimefile::slime_import_preview,
            slimefile::slime_import_apply,
            skill_read,
            soul_feelings,
            genome_preview,
            genome_state,
            genome_day,
            brain_providers,
            brain_config,
            brain_has_key,
            brain_save,
            brain_set_key,
            brain_forget_key,
            brain_chat,
            perms_state,
            perms_grant,
            perms_revoke,
            clipboard_read,
            files_read,
            active_app,
            pointer_nudge,
            screen_capture,
            voice_status,
            voice_install,
            voice_config_save,
            voice_listen,
            voice_stop_listening,
            voice_speak,
            voice_warm,
            voice_stop_speaking,
            learn_choose,
            learn_reward,
            learn_report,
            learn_reset,
            mcp_servers,
            mcp_tools,
            mcp_call,
            log_error,
            quit
        ])
        .run(tauri::generate_context!())
        .expect("failed to run slimy");
}

#[cfg(test)]
mod tool_gate {
    use super::*;

    #[test]
    fn a_tool_runs_only_with_the_power_and_every_attempt_is_journaled() {
        let dir = std::env::temp_dir().join(format!("slimy-gate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("mcp.json"),
            r#"{"servers":[{"name":"s","command":"true"}]}"#,
        )
        .unwrap();
        let ran = std::cell::Cell::new(false);
        let work = |_s: slimy_mcp::Server| {
            ran.set(true);
            Ok(1)
        };
        assert_eq!(
            gated(&dir, "s", 10, "s: list", work).unwrap_err(),
            "no_permission"
        );
        assert!(!ran.get(), "nothing runs without the power");
        perms::change(&dir, |r| r.grant(slimy_perms::Power::Tool, 0, None))
            .unwrap()
            .unwrap();
        assert_eq!(
            gated(&dir, "other", 11, "o", |_| Ok(1)).unwrap_err(),
            "mcp_unknown"
        );
        assert_eq!(gated(&dir, "s", 12, "s: list", |_| Ok(1)).unwrap(), 1);
        perms::change(&dir, |r| r.revoke(slimy_perms::Power::Tool)).unwrap();
        assert_eq!(
            gated(&dir, "s", 13, "s: list", |_| Ok(1)).unwrap_err(),
            "no_permission"
        );
        let log = perms::load(&dir);
        assert_eq!(log.journal().len(), 4);
        assert_eq!(log.journal()[0].verdict, slimy_perms::Verdict::Denied);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
