//! The screens of any computer as pure rules: which screen the slime lives on, where its play area is, when the setup changed
//! (a screen plugged in, a resolution or scale change), how big the slime should be on this screen, and which part of the
//! window must receive the mouse. No windowing code here, so it is tested without a display.

use serde::{Deserialize, Serialize};

/// A rectangle in physical pixels, in the coordinates of the whole desktop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn right(&self) -> i64 {
        i64::from(self.x) + i64::from(self.w)
    }
    pub fn bottom(&self) -> i64 {
        i64::from(self.y) + i64::from(self.h)
    }
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= f64::from(self.x)
            && py >= f64::from(self.y)
            && px < self.right() as f64
            && py < self.bottom() as f64
    }
    fn valid(&self) -> bool {
        self.w > 0 && self.h > 0
    }
    /// Whether `other` lies entirely inside this rectangle.
    fn holds(&self, other: &Rect) -> bool {
        other.valid()
            && i64::from(other.x) >= i64::from(self.x)
            && i64::from(other.y) >= i64::from(self.y)
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }
}

/// One screen: its whole area, the part free of bars and docks, its scale (2.0 on a high density screen) and whether the
/// system calls it the main one.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Screen {
    pub rect: Rect,
    pub work: Rect,
    pub scale: f64,
    pub primary: bool,
}

/// Which screen the user wants the slime on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Preference {
    #[default]
    Primary,
    /// Wherever the pointer is: the slime follows its human from screen to screen.
    Follow,
    /// A fixed screen, by position in the list (0 is the first).
    Index(usize),
}

impl Preference {
    /// Reads the saved text: `primary`, `follow` or a number.
    pub fn parse(text: &str) -> Preference {
        match text.trim().to_lowercase().as_str() {
            "follow" | "cursor" => Preference::Follow,
            other => other
                .parse::<usize>()
                .map_or(Preference::Primary, Preference::Index),
        }
    }

    pub fn text(&self) -> String {
        match self {
            Preference::Primary => "primary".into(),
            Preference::Follow => "follow".into(),
            Preference::Index(i) => i.to_string(),
        }
    }
}

/// The play area of the slime, in physical pixels of the desktop.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

/// The play area of a screen: its work area when that is sensible, else the whole screen.
pub fn world_of(screen: &Screen) -> World {
    let area = if screen.rect.holds(&screen.work) {
        screen.work
    } else {
        screen.rect
    };
    World {
        left: f64::from(area.x),
        top: f64::from(area.y),
        right: area.right() as f64,
        bottom: area.bottom() as f64,
    }
}

fn primary_index(screens: &[Screen]) -> usize {
    screens.iter().position(|s| s.primary).unwrap_or(0)
}

/// The screen holding the point, if any.
pub fn screen_at(screens: &[Screen], px: f64, py: f64) -> Option<usize> {
    screens.iter().position(|s| s.rect.contains(px, py))
}

/// The screen to live on. An index beyond the list falls back to the main screen; `Follow` needs the pointer and keeps
/// the `current` screen when the pointer is nowhere (between screens, or unknown).
pub fn choose(
    screens: &[Screen],
    preference: Preference,
    pointer: Option<(f64, f64)>,
    current: Option<usize>,
) -> usize {
    if screens.is_empty() {
        return 0;
    }
    match preference {
        Preference::Primary => primary_index(screens),
        Preference::Index(i) if i < screens.len() => i,
        Preference::Index(_) => primary_index(screens),
        Preference::Follow => pointer
            .and_then(|(x, y)| screen_at(screens, x, y))
            .or(current.filter(|c| *c < screens.len()))
            .unwrap_or_else(|| primary_index(screens)),
    }
}

/// A number that changes when the setup changes: screens added or removed, moved, resized, rescaled, or their work areas
/// changed (a bar appeared). Equal setups give equal numbers.
pub fn signature(screens: &[Screen]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut mix = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    };
    mix(screens.len() as u64);
    for s in screens {
        for r in [s.rect, s.work] {
            mix(r.x as u32 as u64);
            mix(r.y as u32 as u64);
            mix(u64::from(r.w));
            mix(u64::from(r.h));
        }
        mix(s.scale.to_bits());
        mix(u64::from(s.primary));
    }
    h
}

/// How big the slime is on this screen, as a multiplier of its usual size: bigger on tall (or dense) screens, smaller on
/// small ones, in steps of a twentieth so it does not jitter. `logical_height` is the play area's height in logical pixels.
pub fn size_scale(logical_height: f64) -> f32 {
    if !logical_height.is_finite() || logical_height <= 0.0 {
        return 1.0;
    }
    let raw = (logical_height / 1000.0).clamp(0.8, 1.4);
    ((raw * 20.0).round() / 20.0) as f32
}

/// Moves the slime to another screen only after the pointer has stayed there a while, so brushing across an edge does not
/// make it jump around.
#[derive(Clone, Copy, Debug, Default)]
pub struct Follower {
    candidate: Option<(usize, f64)>,
}

/// Seconds the pointer must rest on another screen before the slime follows.
pub const FOLLOW_AFTER: f64 = 2.5;

impl Follower {
    /// Returns the screen to live on now, given the one it is on, the pointer's screen and the time in seconds.
    pub fn update(&mut self, now: f64, current: usize, pointer_screen: Option<usize>) -> usize {
        match pointer_screen {
            Some(p) if p != current => match self.candidate {
                Some((c, since)) if c == p => {
                    if now - since >= FOLLOW_AFTER {
                        self.candidate = None;
                        p
                    } else {
                        current
                    }
                }
                _ => {
                    self.candidate = Some((p, now));
                    current
                }
            },
            _ => {
                self.candidate = None;
                current
            }
        }
    }
}

/// The part of the window that takes the mouse, in window pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HitRegion {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl HitRegion {
    /// Whether a pointer at (`px`, `py`) window pixels is inside. An empty region takes nothing.
    pub fn takes(&self, px: f64, py: f64) -> bool {
        self.w > 0
            && self.h > 0
            && px >= f64::from(self.x)
            && py >= f64::from(self.y)
            && px < f64::from(self.x) + f64::from(self.w)
            && py < f64::from(self.y) + f64::from(self.h)
    }

    /// The same region in physical pixels for a window scale factor.
    pub fn scaled(&self, scale: f64) -> HitRegion {
        let s = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        HitRegion {
            x: (f64::from(self.x) * s).floor() as i32,
            y: (f64::from(self.y) * s).floor() as i32,
            w: (f64::from(self.w) * s).ceil() as i32,
            h: (f64::from(self.h) * s).ceil() as i32,
        }
    }
}

/// Decides click-through from where the pointer is, with a small margin so the edge of the slime is easy to grab and the
/// switch does not flicker. Returns `Some(ignore)` only when the answer changes. A fresh gate takes clicks until told
/// otherwise: safer than hiding a button that was not found yet.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClickThrough {
    ignoring: bool,
}

/// Extra pixels around the region where the window already takes the mouse.
pub const GRAB_MARGIN: i32 = 6;

impl ClickThrough {
    pub fn update(
        &mut self,
        region: &HitRegion,
        pointer_in_window: Option<(f64, f64)>,
    ) -> Option<bool> {
        let want_ignore = match pointer_in_window {
            Some((x, y)) => {
                let wide = HitRegion {
                    x: region.x - GRAB_MARGIN,
                    y: region.y - GRAB_MARGIN,
                    w: region.w + 2 * GRAB_MARGIN,
                    h: region.h + 2 * GRAB_MARGIN,
                };
                !wide.takes(x, y)
            }
            // No pointer position known: take the clicks rather than swallow them.
            None => false,
        };
        (want_ignore != self.ignoring).then(|| {
            self.ignoring = want_ignore;
            want_ignore
        })
    }
}
