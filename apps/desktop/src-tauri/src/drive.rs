//! Development tool, debug builds only: drives the real desktop through the RemoteDesktop portal (pointer and keyboard)
//! so the app can be tested the way a person uses it. Commands are appended to a file, one per line, and answered in
//! another file. Never part of a release build.
//!
//! Commands: `move X Y`, `click left|right|double`, `type TEXT`, `key return|escape|backspace`, `shot PATH`, `pos`, `quit`.

use std::io::Write;
use std::time::Duration;

use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions};

const LEFT: i32 = 0x110;
const RIGHT: i32 = 0x111;

fn keysym(c: char) -> i32 {
    match c {
        '\n' => 0xff0d,
        '\t' => 0xff09,
        c if (c as u32) < 0x100 => c as i32,
        c => 0x0100_0000 + c as i32,
    }
}

pub async fn run(commands: &str, answers: &str) -> Result<(), String> {
    let proxy = RemoteDesktop::new().await.map_err(|e| e.to_string())?;
    let session = proxy
        .create_session(Default::default())
        .await
        .map_err(|e| e.to_string())?;
    proxy
        .select_devices(
            &session,
            SelectDevicesOptions::default()
                .set_devices(Some(DeviceType::Pointer | DeviceType::Keyboard)),
        )
        .await
        .map_err(|e| e.to_string())?;
    let started = proxy
        .start(&session, None, Default::default())
        .await
        .map_err(|e| e.to_string())?
        .response()
        .map_err(|e| e.to_string())?;
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(answers)
        .map_err(|e| e.to_string())?;
    writeln!(log, "READY devices {:?}", started.devices()).ok();
    let mut done = 0usize;
    loop {
        tokio::time::sleep(Duration::from_millis(120)).await;
        let text = std::fs::read_to_string(commands).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        while done < lines.len() {
            let line = lines[done];
            done += 1;
            let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
            let reply: Result<String, String> = async {
                match word {
                    "quit" => {
                        let _ = session.close().await;
                        Ok("bye".into())
                    }
                    "pos" => Ok(format!("{:?}", crate::shell_pointer())),
                    "move" => {
                        let nums: Vec<f64> = rest
                            .split_whitespace()
                            .filter_map(|n| n.parse().ok())
                            .collect();
                        let [tx, ty] = nums[..] else {
                            return Err("move X Y".into());
                        };
                        for _ in 0..40 {
                            let (px, py) = crate::shell_pointer().ok_or("no pointer")?;
                            let (dx, dy) = (tx - px, ty - py);
                            if dx.abs() < 1.0 && dy.abs() < 1.0 {
                                break;
                            }
                            let (sx, sy) = (dx.clamp(-120.0, 120.0), dy.clamp(-120.0, 120.0));
                            proxy
                                .notify_pointer_motion(&session, sx, sy, Default::default())
                                .await
                                .map_err(|e| e.to_string())?;
                            tokio::time::sleep(Duration::from_millis(25)).await;
                        }
                        Ok(format!("{:?}", crate::shell_pointer()))
                    }
                    "click" => {
                        let (button, times) = match rest {
                            "right" => (RIGHT, 1),
                            "double" => (LEFT, 2),
                            _ => (LEFT, 1),
                        };
                        for _ in 0..times {
                            proxy
                                .notify_pointer_button(
                                    &session,
                                    button,
                                    KeyState::Pressed,
                                    Default::default(),
                                )
                                .await
                                .map_err(|e| e.to_string())?;
                            tokio::time::sleep(Duration::from_millis(60)).await;
                            proxy
                                .notify_pointer_button(
                                    &session,
                                    button,
                                    KeyState::Released,
                                    Default::default(),
                                )
                                .await
                                .map_err(|e| e.to_string())?;
                            tokio::time::sleep(Duration::from_millis(90)).await;
                        }
                        Ok("clicked".into())
                    }
                    "type" => {
                        for c in rest.chars() {
                            proxy
                                .notify_keyboard_keysym(
                                    &session,
                                    keysym(c),
                                    KeyState::Pressed,
                                    Default::default(),
                                )
                                .await
                                .map_err(|e| e.to_string())?;
                            proxy
                                .notify_keyboard_keysym(
                                    &session,
                                    keysym(c),
                                    KeyState::Released,
                                    Default::default(),
                                )
                                .await
                                .map_err(|e| e.to_string())?;
                            tokio::time::sleep(Duration::from_millis(25)).await;
                        }
                        Ok(format!("typed {} characters", rest.chars().count()))
                    }
                    "key" => {
                        let sym = match rest {
                            "return" => 0xff0d,
                            "escape" => 0xff1b,
                            "backspace" => 0xff08,
                            "tab" => 0xff09,
                            _ => return Err("unknown key".into()),
                        };
                        proxy
                            .notify_keyboard_keysym(
                                &session,
                                sym,
                                KeyState::Pressed,
                                Default::default(),
                            )
                            .await
                            .map_err(|e| e.to_string())?;
                        proxy
                            .notify_keyboard_keysym(
                                &session,
                                sym,
                                KeyState::Released,
                                Default::default(),
                            )
                            .await
                            .map_err(|e| e.to_string())?;
                        Ok("key sent".into())
                    }
                    "shot" => {
                        use base64::Engine;
                        let text = crate::screen::capture().await?;
                        let bytes = base64::engine::general_purpose::STANDARD
                            .decode(text)
                            .map_err(|e| e.to_string())?;
                        std::fs::write(rest, &bytes).map_err(|e| e.to_string())?;
                        Ok(format!("saved {} bytes", bytes.len()))
                    }
                    _ => Err(format!("unknown command {word}")),
                }
            }
            .await;
            match reply {
                Ok(text) => {
                    writeln!(log, "DONE {done} {text}").ok();
                    if word == "quit" {
                        return Ok(());
                    }
                }
                Err(e) => {
                    writeln!(log, "FAIL {done} {e}").ok();
                }
            }
        }
    }
}
