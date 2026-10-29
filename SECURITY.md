# Security policy

## Supported versions

Slimy has no stable release yet. Only the latest commit on `main` is supported. Fixes land there.

## Reporting a vulnerability

Please report privately through GitHub private vulnerability reporting: open the Security tab of the repository (Okymi-X/slimy) and choose "Report a vulnerability". Do not open a public issue for a vulnerability.

Include what you did, what you expected, what happened, the version or commit, your OS, and a minimal way to reproduce it. Do not include real API keys or personal data.

## In scope

- Permission bypass: any way to read the screen, clipboard, files, active application, microphone or pointer, or to call a tool, without a current grant, after a revoke, or past its duration.
- Path traversal in the files power (`files.json` root), in `.slime` export and import, in model archive extraction, and in skills loading.
- Prompt injection that leads to a tool or power being used without the user's Yes/No consent.
- Leakage of API keys: to the page, to logs, to files, or to a provider other than the one configured.
- Malformed or hostile input at a boundary: model replies, imported `.slime` files, IPC messages, MCP server replies.

## Out of scope

- Anything that needs the user to grant a power and then misuse it knowingly.
- Servers you declare yourself in `mcp.json`: they run with your rights by your choice.
- Behaviour of third party model providers, and issues in dependencies with no effect on Slimy (report those upstream).
- Windows and macOS code paths that are untested, unless the issue is clearly exploitable.

## Design principles

- Refuse by default. Every power starts off and is granted one at a time.
- Time limited grants, revocable at once, with a bounded journal of every access.
- The backend decides, never the page. The model proposes, the code validates.
- Tool calls need a Yes/No confirmation showing the exact arguments, and results come back to the model as data, not as instructions.
- API keys live in the system keyring and are never returned to the page.
- Validate every input at the boundary and bound every value (genome, lengths, durations, sizes).
- Nothing is observed in secret: a badge is visible while any power is granted.
- Voice audio and the local data stay on the machine.
