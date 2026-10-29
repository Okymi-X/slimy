# Third party components

Slimy core is MIT. The items below are not part of it.

## Models (downloaded at run time, not stored in this repository)

| Component | Source | Licence | Attribution |
|---|---|---|---|
| Whisper base and small (speech to text, sherpa-onnx exports) | OpenAI Whisper | MIT | Keep the notice "Copyright (c) 2022 OpenAI" if redistributed |
| Kokoro multi-lang v1.0 (text to speech, voices.bin) | hexgrad, Kokoro-82M | Apache-2.0 | Credit hexgrad and the Kokoro project; keep the licence text if redistributed |
| sherpa-onnx (engine and model packaging) | k2-fsa | Apache-2.0 | Keep the licence text when redistributing binaries |
| espeak-ng-data (inside the Kokoro archive) | espeak-ng | GPL-3.0-or-later | See below |

Models are fetched from the k2-fsa sherpa-onnx GitHub releases. Check each voice's upstream licence file before bundling any model in an installer.

## French voice and espeak-ng (GPL-3)

The French voice phonemises text with the `espeak-rs` crate, which links espeak-ng (GPL-3.0-or-later) although the crate declares MIT. A binary that includes it is GPL-3 as a whole. Publish and ship the French voice as an optional, separable part (own build feature or package) with its source offered under GPL-3. It is the `french-voice` Cargo feature of `slimy-desktop` and is off by default, so a default build contains no GPL code. `deny.toml` documents this as the only exception.

## Rust dependencies

About 700 crates, almost all MIT, Apache-2.0 or dual licensed. Notable terms:

- MPL-2.0: cssparser, selectors, dtoa-short, option-ext (pulled by Tauri). File level copyleft, no action unless those files are modified.
- BSL-1.0: clipboard-win, error-code. CDLA-Permissive-2.0: webpki-roots. bzip2-1.0.6: libbz2 binding. All permissive.
- Tauri 2, the sherpa-onnx crate, rustls, reqwest, oo7, xcap, arboard, rodio, cpal: permissive.

Check policy with `cargo deny check` (see `deny.toml`).

## Front end

The web UI (apps/desktop/web) is plain JavaScript with no npm dependencies and no bundled third party code.
