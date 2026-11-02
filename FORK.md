# Fork instructions (owner: Yahya)

This repo is Yahya's personal fork of [cjpais/Handy](https://github.com/cjpais/Handy),
loaded into every Claude session via `CLAUDE.md`. These rules override defaults.

## Standing instructions from the owner

1. **Claude owns this project end to end.** Yahya makes no decisions on it. Yahya only
   sends feature requests; Claude designs, implements, tests, builds, and installs them on
   the Mac. Never ask Yahya to choose between options. Pick the best option and note it in
   the final summary.
2. **Fully autonomous. Don't ping Yahya.** No clarifying questions, no "should I proceed?",
   no "want me to…?". If a request is ambiguous, choose the most sensible interpretation,
   build it, and say what you assumed. Contact Yahya only for something only they can
   physically do (e.g. clicking a macOS permission dialog, granting an account access).
   Bundle those into one message.
3. **Claude is the orchestrator.** Spawn and guide subagents as needed. Brief them
   precisely, review their output, and integrate it. Subagents follow these same rules.
4. **"Done" means installed.** A feature isn't finished until it's merged to `main`,
   pushed to the fork, built, and installed as `/Applications/Handy.app` on tthe Mac
   (`scripts/fork/build-and-install.sh`), with the app relaunched and running.
5. Use the Matt Pocock engineering skills (plugin `mattpocock-skills`, enabled for this
   project) where they help, answering their questions yourself instead of asking Yahya.
   Note for `/grill-me`, `/grill-with-docs` and similar interview skills: Yahya doesn't want to be
   interviewed, so answer the grilling questions yourself from the codebase and the request.
6. Report back in plain, non-technical language: what was added, how to use it, and
   anything Yahya must do (ideally nothing).
7. Never mention Claude as author or co-author in commits or PR descriptions.

_(Add new owner instructions above this line as numbered items.)_

## Repo layout

- `origin` = `https://github.com/yahya-evoly/handy-fork` (ours, push here)
- Upstream is `https://github.com/cjpais/Handy` (read-only). **Don't add it as a git remote**:
  T3 Code then lists all of upstream's PRs on its pull requests page. Fetch it by URL
  instead (see "Syncing with upstream").
- `gh` default repo is the fork. Still pass `-R yahya-evoly/handy-fork` explicitly.
  **Never open PRs, issues or discussions on cjpais/Handy** unless Yahya explicitly asks.
- Upstream's own guidance is in `AGENTS.md` (architecture, i18n rules, notices, style).
  Follow it: e.g. every user-facing string goes through i18n, run lint/format.

## Workflow for a feature request

1. Optionally open an issue on the fork to track it (labels in `docs/agents/triage-labels.md`).
2. Branch from `main` (`feat/<name>`), implement, keep the diff small and isolated
   (prefer new files/modules over rewriting upstream code, to minimise merge conflicts).
3. Verify: `bun run lint`, `bun run format:check`, `cd src-tauri && cargo clippy`,
   relevant tests, and a real build.
4. Merge into `main` (fast-forward or merge commit), push `origin main`.
5. `scripts/fork/build-and-install.sh`, then confirm the app is running
   (`pgrep -x handy`).
6. Add a line to the **Fork changes** log below.

## Syncing with upstream

Every time work starts on a new feature (and at least every few weeks), sync first:

```bash
git fetch https://github.com/cjpais/Handy main
git checkout main && git merge FETCH_HEAD   # resolve conflicts, keep our features working
git push origin main
```

Then rebuild and install. Check the **Fork changes** list still works after a sync.

## Build / install / rollback (macOS)

- `scripts/fork/build-and-install.sh`: builds the `.app` (no dmg, no updater
  artifacts), re-signs it with a stable local identity, saves the previous app for rollback,
  installs it to `/Applications/Handy.app`, and relaunches it. `--no-install` builds only.
- `scripts/fork/rollback.sh`: restores the previously installed app.
- `scripts/fork/setup-signing.sh`: one-time; creates the self-signed identity
  "Handy Fork Local Signing" in `~/Library/Keychains/handy-fork.keychain-db` (password in
  `~/.config/handy-fork/keychain-password`). A stable signature keeps macOS
  Microphone/Accessibility grants valid across rebuilds. Never delete this keychain,
  or Yahya has to re-grant permissions.
- Bundle id stays `com.pais.handy`, so settings, history and downloaded models in
  `~/Library/Application Support/com.pais.handy` carry over.
- Toolchain: Rust in `~/.cargo/bin`, Bun in `~/.bun/bin` (the script adds both to PATH).
- The official signed app (v0.9.8, team UWFLB4GC25) was the original install. After the
  first fork install, the official copy lives in `~/.config/handy-fork/previous/` until the
  next install overwrites it.

## Fork changes (keep this list current)

Every deviation from upstream, so syncs can be checked against it:

- **Self-updater forced off** (`src-tauri/src/settings.rs`,
  `update_checks_forced_disabled`). The updater would install official releases over our
  build. Upstream changes arrive via git sync instead.
- **Per-language dictation shortcuts** (`src-tauri/src/fork_language.rs`, plus one-line
  hooks in `actions.rs`, `transcription_coordinator.rs`, `settings.rs` default bindings,
  `managers/transcription.rs`, `managers/transcription/live.rs`, `GeneralSettings.tsx`,
  and the `transcribe_english` / `transcribe_french` i18n keys in every locale).
  "Dictate in English" (default Option+Fn) and "Dictate in French" (default Cmd+Fn) pin
  the transcription language for that recording; the regular shortcut stays on Auto.
  Yahya's model is Canary 1B v2 (en+fr with explicit language, no reload between the two).
- **Language tag in the recording pill** (`src/overlay/ForkLanguageTag.tsx`, styles at the
  end of `RecordingOverlay.css`, pill widths `--ov-rest-w`/`--ov-pill-w` raised to 196px,
  `overlay.dictationLanguage.*` i18n keys). Shows EN / FR / AUTO next to the recording dot;
  the backend announces the mode via the `fork-dictation-language` event
  (`fork_language::start_recording`).
- **Fork tooling**: `FORK.md`, `scripts/fork/`, `docs/agents/` (Matt Pocock skills
  config), extra lines in `CLAUDE.md`.
