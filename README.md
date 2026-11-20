# DayDitto for Mac

A thin native shell ([Tauri 2](https://v2.tauri.app)) around the live web app at
**https://dayditto.prototypolab.com**. The app ships no planner code of its own: it shows a
local splash, then loads the site in a WKWebView, and adds what a browser tab can't do:

- **Dock badge** with today's open items (tasks plus scheduled subtasks).
- **Quick capture from anywhere**: **⌃⌥D** (Control+Option+D) toggles a small floating window
  (`/quick`) that files a thought into a list. Enter saves and closes, ⌘Enter saves and stays,
  Esc closes; clicking elsewhere dismisses it. The shortcut is fixed in v1; if another app
  already owns it, DayDitto runs without it (logged).
  The window is a non-activating floating panel ([tauri-nspanel](https://github.com/ahkohd/tauri-nspanel)):
  it takes the keyboard without activating DayDitto, so the main window stays as it was (hidden
  stays hidden, in the background stays there) and closing it leaves you in the app you were in.
  It shows on every Space, over full-screen apps too. A saved task reaches the main window right
  away (`quick_saved` → `tasks-changed` event); the main window also refreshes when it regains focus.
- **Zoom** (main window only; quick capture stays at 100%): View → Zoom In ⌘=, Zoom Out ⌘-,
  Actual Size ⌘0, from 50% to 200% in 10% steps. The level is saved in `zoom.json` in the app
  config dir (`~/Library/Application Support/com.prototypolab.dayditto/`) and restored at launch;
  it is applied again after every page load, since WKWebView may reset it on navigation.
- **Native menu**: Edit (undo/copy/paste for text fields), View (Zoom, Reload, Full Screen), Go (Today ⌘T,
  Week ⇧⌘W, Focus Mode ⇧⌘F, Quick Dump ⌘N, Settings ⌘,), Window, Help, plus *Check for Updates…*
  and *Launch at Login* in the app menu.
- Closing the window keeps DayDitto running (click the Dock icon to reopen; ⌘Q quits).
  Window size and position are remembered.
- **Auto-updates** from GitHub Releases: a quiet check 10 s after launch and every 6 hours.
- Online only: without a connection the splash says "You're offline" and retries every 5 s.

Bundle id `com.prototypolab.dayditto`, universal binary (Apple Silicon + Intel), macOS 11+.

## Security model

- All content is remote (the DayDitto origin) except the bundled splash (`dist/`), which has a
  `'self'`-only CSP and no IPC access.
- **Only four commands** are exposed, and only to `https://dayditto.prototypolab.com/*` in the
  `main` and `quick` windows (`src-tauri/capabilities/default.json`): `set_badge`, `hide_quick`,
  `quick_saved` (only emits `tasks-changed` to the main window), `open_external` (https URLs
  only). The web page may also listen to events (`menu`, `tasks-changed`, `quick-shown`).
  No file system, shell, process, updater or dialog access from JavaScript. `build.rs` lists the
  commands in the app manifest, so they need an explicit permission; anything else is denied.
- Navigation: the webview stays on the DayDitto origin. Any other http(s) or mailto link opens in
  the default browser/app and the navigation is cancelled; `target=_blank` and `window.open`
  never create webviews.
- Updates are signed (minisign key in `tauri.conf.json`); the app is signed with a Developer ID
  and notarized.

## How sign-in works

Google refuses sign-in inside embedded webviews, so it happens in the system browser:

1. On `/login` the page sees `DayDitto-Desktop` in the user agent and calls Supabase
   `signInWithOAuth({ redirectTo: "dayditto://auth/callback", skipBrowserRedirect: true })`. That
   stores the PKCE verifier as a cookie **in this webview**, and the page asks the app
   (`open_external`) to open the Google URL in the browser.
2. Google → Supabase → `dayditto://auth/callback?code=…`. macOS hands the link to the app
   (deep-link plugin; a second launch is forwarded by single-instance).
3. The app navigates the main window to
   `https://dayditto.prototypolab.com/auth/callback?code=…` and brings it to the front. The web
   app's route exchanges the code (with the verifier cookie) and redirects to `/`.

The session cookies live in the app's WKWebView data store, shared by both windows. Other
`dayditto://` links are ignored, and a link is handled only once (an auth code works once).

## Develop

Needs a Mac with Rust (stable) and Node 22.

```bash
npm ci
npm run tauri dev
```

Deep links only work in a built, installed app (the URL scheme is registered from the bundle's
Info.plist), so test sign-in with a build from the *Check* workflow (below) or `npm run tauri build`.

## Release

1. Bump the version in `src-tauri/tauri.conf.json` **and** `src-tauri/Cargo.toml`.
2. Commit, tag and push: `git tag v0.2.0 && git push origin main v0.2.0`.
3. The *Release* workflow builds a universal DMG, signs and notarizes it, and publishes a GitHub
   Release with the DMG, the updater bundle (`.app.tar.gz` + `.sig`) and `latest.json`. It fails
   early if the tag and the two versions don't match. Running it manually (no tag) releases
   `v<version from tauri.conf.json>`.

Installed apps poll `https://dayditto.prototypolab.com/desktop/latest.json`, which redirects to
this repo's `releases/latest/download/latest.json`; the website's `/download/mac` redirects to the
latest DMG.

Required repository secrets (names only): `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
`APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`,
`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (`GITHUB_TOKEN` is automatic).

The *Check* workflow runs `cargo check` and a debug build on every push to `main` and on pull
requests. Run it manually (*Actions → Check → Run workflow*) to get an **unsigned** universal DMG
as an artifact; it's ad-hoc signed and not notarized, so open it with right-click → Open (or
`xattr -cr /Applications/DayDitto.app`).

## Known limits

- Online only; there's no offline copy of your planner.
- The quick capture shortcut is fixed (⌃⌥D).
- Losing the connection after the site has loaded shows WebKit's error page; use View → Reload.
- `Cargo.lock` is not committed yet: commit the one from the first CI build for reproducible builds.

## Layout

```
dist/                     splash / offline page (index.html, splash.css, splash.js)
src-tauri/
  tauri.conf.json         windows (created from Rust with `create: false`), bundle, updater, deep link
  capabilities/default.json   the only permissions, for the DayDitto origin
  build.rs                app manifest: the four commands
  src/lib.rs              plugins, windows, user agent, close-to-hide, Dock reopen
  src/commands.rs         set_badge, hide_quick, quick_saved, open_external
  src/navigation.rs       what stays in the webview, what opens in the browser
  src/deep_link.rs        dayditto://auth/callback → /auth/callback
  src/quick.rs            the quick capture window (an NSPanel on macOS)
  src/shortcut.rs         ⌃⌥D
  src/menu.rs             menu bar, Go actions → `menu` event
  src/zoom.rs             main window zoom: levels, zoom.json, re-applied after page loads
  src/updater.rs          checks, dialog, install and restart
```
