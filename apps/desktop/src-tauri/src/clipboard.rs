//! Reads the clipboard text through `arboard` (Windows, macOS, X11, Wayland), then the Linux command line tools. Only reached after the
//! `clipboard` power has been checked.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Longest text returned, in characters.
const MAX_CHARS: usize = 2000;
const MAX_BYTES: u64 = 64 * 1024;
const TIMEOUT: Duration = Duration::from_secs(2);

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn run(program: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut out = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = out.by_ref().take(MAX_BYTES).read_to_end(&mut bytes);
        bytes
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait().ok()? {
            Some(s) => break s,
            None if start.elapsed() > TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    let bytes = reader.join().ok()?;
    status
        .success()
        .then(|| String::from_utf8_lossy(&bytes).into_owned())
}

/// The clipboard text, cut to a safe size; `None` when it holds no text or no tool is available.
pub fn read() -> Option<String> {
    let text = from_library()
        .or_else(|| run("wl-paste", &["--no-newline", "--type", "text/plain"]))
        .or_else(|| run("xclip", &["-o", "-selection", "clipboard"]))?;
    Some(tidy(&text))
}

/// The library call can wait for ever on a clipboard owner that does not answer: it runs on its own thread with a deadline.
fn from_library() -> Option<String> {
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = send.send(
            arboard::Clipboard::new()
                .and_then(|mut c| c.get_text())
                .ok(),
        );
    });
    receive.recv_timeout(TIMEOUT).ok().flatten()
}

fn tidy(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .take(MAX_CHARS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_is_cleaned_and_bounded() {
        assert_eq!(tidy("a\u{0}b\u{1b}[31mc\nd"), "ab[31mc\nd");
        assert_eq!(tidy(&"x".repeat(5000)).chars().count(), MAX_CHARS);
    }

    #[test]
    fn a_missing_program_gives_nothing() {
        assert_eq!(run("definitely-not-a-program", &[]), None);
        assert_eq!(run("false", &[]), None);
        assert_eq!(run("echo", &["hi"]).as_deref(), Some("hi\n"));
    }
}
