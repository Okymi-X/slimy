//! Moves the real pointer a little, the way each system allows it. On Linux the RemoteDesktop portal
//! asks the user once per run in a system dialog; Windows and macOS use the system input API (macOS
//! needs the accessibility permission). Only reached after the `pointer` power has been checked.

/// Farthest one call may move the pointer on each axis, in pixels.
pub const MAX_STEP: i32 = 300;

/// A requested move, bounded.
pub fn clamp_step(dx: i32, dy: i32) -> (i32, i32) {
    (dx.clamp(-MAX_STEP, MAX_STEP), dy.clamp(-MAX_STEP, MAX_STEP))
}

#[cfg(target_os = "linux")]
mod system {
    use std::time::Duration;

    use ashpd::desktop::remote_desktop::{DeviceType, RemoteDesktop, SelectDevicesOptions};
    use ashpd::desktop::Session;
    use tauri::async_runtime::Mutex;

    /// How long the user has to answer the system dialog.
    const CONSENT: Duration = Duration::from_secs(60);
    /// The pointer moves in small pieces so it glides instead of jumping.
    const PIECES: i32 = 12;

    static SESSION: Mutex<Option<(RemoteDesktop, Session<RemoteDesktop>)>> = Mutex::const_new(None);

    async fn open() -> Result<(RemoteDesktop, Session<RemoteDesktop>), String> {
        let proxy = RemoteDesktop::new()
            .await
            .map_err(|_| "pointer_unsupported".to_string())?;
        let session = proxy
            .create_session(Default::default())
            .await
            .map_err(|_| "pointer_unsupported".to_string())?;
        proxy
            .select_devices(
                &session,
                SelectDevicesOptions::default().set_devices(Some(DeviceType::Pointer.into())),
            )
            .await
            .map_err(|_| "pointer_unsupported".to_string())?;
        let asked = tokio::time::timeout(CONSENT, async {
            proxy
                .start(&session, None, Default::default())
                .await?
                .response()
        })
        .await;
        let granted = match asked {
            Ok(Ok(devices)) => devices,
            Ok(Err(_)) => return Err("pointer_denied".into()),
            Err(_) => {
                let _ = session.close().await;
                return Err("pointer_timeout".into());
            }
        };
        if !granted.devices().contains(DeviceType::Pointer) {
            let _ = session.close().await;
            return Err("pointer_denied".into());
        }
        Ok((proxy, session))
    }

    pub async fn nudge(dx: i32, dy: i32) -> Result<(), String> {
        let mut slot = SESSION.lock().await;
        if slot.is_none() {
            *slot = Some(open().await?);
        }
        let (proxy, session) = slot.as_ref().expect("opened above");
        for i in 1..=PIECES {
            let share = |total: i32| f64::from(total * i / PIECES - total * (i - 1) / PIECES);
            if proxy
                .notify_pointer_motion(session, share(dx), share(dy), Default::default())
                .await
                .is_err()
            {
                *slot = None;
                return Err("pointer_denied".into());
            }
            tokio::time::sleep(Duration::from_millis(12)).await;
        }
        Ok(())
    }

    /// Ends the portal session, so the dialog is asked again next time.
    pub async fn close() {
        if let Some((_, session)) = SESSION.lock().await.take() {
            let _ = session.close().await;
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod system {
    use enigo::{Coordinate, Enigo, Mouse, Settings};

    pub async fn nudge(dx: i32, dy: i32) -> Result<(), String> {
        tauri::async_runtime::spawn_blocking(move || {
            let mut enigo =
                Enigo::new(&Settings::default()).map_err(|_| "pointer_denied".to_string())?;
            enigo
                .move_mouse(dx, dy, Coordinate::Rel)
                .map_err(|_| "pointer_denied".to_string())
        })
        .await
        .map_err(|_| "pointer_denied".to_string())?
    }

    pub async fn close() {}
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod system {
    pub async fn nudge(_: i32, _: i32) -> Result<(), String> {
        Err("pointer_unsupported".into())
    }
    pub async fn close() {}
}

/// Moves the pointer by (dx, dy) pixels, bounded by `MAX_STEP`.
pub async fn nudge(dx: i32, dy: i32) -> Result<(), String> {
    let (dx, dy) = clamp_step(dx, dy);
    system::nudge(dx, dy).await
}

pub async fn close() {
    system::close().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_never_exceeds_the_limit() {
        assert_eq!(clamp_step(10, -20), (10, -20));
        assert_eq!(clamp_step(i32::MAX, i32::MIN), (MAX_STEP, -MAX_STEP));
        assert_eq!(clamp_step(-301, 301), (-MAX_STEP, MAX_STEP));
    }
}
