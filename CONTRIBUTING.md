# Contributing to Slimy

Thanks for helping. Windows and macOS help is especially welcome: that code exists but has never been built or run by the maintainers, so reports and fixes from those systems are valuable.

## Set up

Follow "Build and run" in `README.md`: system packages, a Rust toolchain, the `wasm32-unknown-unknown` target, `wasm-bindgen-cli`, then `./scripts/build-web.sh` and `cargo build -p slimy-desktop --features custom-protocol`. Node 22 is used for the web tests. Read `docs/RULES.md` and `docs/ARCHITECTURE.md` before a change.

## Run the checks CI runs

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/build-web.sh
cd apps/desktop/web && node --test tests/*.test.mjs
```

The headless scenario tests (`dev/*-test.mjs` in `apps/desktop/web`) run the real page in Chrome. Serve the folder on `HARNESS_PORT` (CI uses 8765) with `python3 -m http.server`, then run for example `xvfb-run -a node dev/panels-test.mjs`. See the Testing section of `README.md`.

## Code rules

- Validate every input at the boundary: model replies, imported `.slime` files, IPC messages.
- No secrets in code or logs. API keys go to the system keyring.
- Refuse by default: reading the screen, apps, files or clipboard goes through `slimy-perms`.
- One responsibility per module and function. Keep simulation, rendering, I/O and interface apart.
- Search before writing: reuse helpers and dependencies that exist.
- `slimy-core` and `slimy-body` have no I/O, no system clock and no global randomness. Time and seeds are parameters.
- Clippy clean, rustfmt, no `unwrap` outside tests. Bound every genome value on input and after every change.
- The model proposes, the code decides: validate outputs, bound deltas, keep a speaking budget. Always keep a fallback that works without a model.
- No emoji and no em or en dashes in code, comments, docs or interface text. No comments that repeat the code, no dead code.
- Add tests with the change. Rule changes need unit tests, and anything touching genomes needs property tests.

## Add a skin or form

Forms are chosen per provider in `apps/desktop/web/forms.js`: `FORM_OF_PROVIDER`, plus the silhouette (`SHAPES`), base proportions (`BASES`), colours (`COLORS`), limbs (`LIMBS`), screen margin (`INSET`) and, for dark gel, the ink (`INK`). Check it in `apps/desktop/web/dev/preview.html?form=NAME`, extend `tests/forms.test.mjs`, and confirm the form can be grabbed by its limbs. Body parameters can also be set as data (`slimy-body` `BodyParams`).

## Add a language

Strings live in `apps/desktop/web/i18n.js` (currently French and English, anything else falls back to English). Add a new block with every key the existing ones have, keep the same placeholders such as `{user}` and `{slime}`, and check that names and short answers still validate.

## Add a learned decision

In `crates/slimy-learn/src/lib.rs`, add a `Decision` constant and its entry in `DECISIONS`, with a short list of safe options written in code. Then add the reward on the page side in `apps/desktop/web/learn.js`. The page names the decision and never the value. Learning only changes bounded settings, never text or powers.

## Pull request checklist

- [ ] The checks above pass locally.
- [ ] New behaviour has tests, and a bug fix has a test that failed before.
- [ ] No new emoji, em or en dashes, secrets, or personal paths.
- [ ] Anything that reads user data goes through `slimy-perms`.
- [ ] Docs updated if behaviour changed (`README.md`, `docs/`, `THIRD_PARTY.md` for new dependencies or models).
- [ ] Dependencies pass `cargo deny check`.
- [ ] The PR says what you tested and on which OS.

## Commit messages

Imperative and plain, one short subject line, for example "Fix the swallowed cursor arrow position". Add a body when the reason is not obvious.

## Licence of contributions

By contributing you agree that your work is released under the MIT licence of this repository. The exception is the optional `french-voice` feature, which links espeak-ng and is GPL-3: a build with that feature is GPL-3 as a whole, and the default build must stay free of GPL code.

## Security

Do not report vulnerabilities in public issues. See `SECURITY.md`.
