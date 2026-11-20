# 08 Install and update from GitHub Releases

Status: ready-for-agent
Blocked by: 02, 03

## Why

Spec Decisions 4, 9 and 10, "Install and update flow". A plugin lives in
its author's repository; a version is a GitHub Release with a semver tag.
Updates are automatic unless they ask for new permissions; conflicts
between plugins are settled per part by the user, not refused. Official
plugins ship inside the binary, and authors install their own builds
from a local folder.

## What to build

1. New crate `otty-plugin-releases` in `crates/plugins/releases` with a
   `README.md`. No `iced`, no `crates/app/`.
2. A GitHub URL (`https://github.com/<owner>/<repo>`, with or without
   `.git` or a trailing slash) → the releases list through the GitHub
   REST API with `ureq`, no login, OTTY's `User-Agent`; a plain error for
   rate limits, a private or missing repository, no network.
3. Tags: semver with an optional leading `v`, parsed and ordered with the
   `semver` crate (approved, already in `Cargo.lock`); others ignored;
   pre-releases skipped unless the user asks. Newest compatible first:
   download only `otty-plugin.toml`, check that this OTTY knows every
   contract version of its `[api]` (issue 02) and that its version equals
   the tag. None compatible: "Needs a newer OTTY" naming the contract.
4. "Official": the repository's owner equals one constant holding the
   official GitHub organization (`otty-shell` today; the owner may rename
   it, so it is named in this one place only).
5. Install: check the manifest (issue 02); refuse a plugin id already
   installed from another repository (the error names it); for each part
   whose agent id, provider id or provider host another installed plugin
   (or a built-in reader) already serves, give the conflict to the
   dialog instead of refusing, and record the user's answer: the chosen
   part is on, the losing part is off. Download the `.wasm` (at most 20
   MiB) and icons (each at most 16 KiB, parsed with `usvg`, approved
   2026-11-20 as a direct dependency; no `<script>`, no external `href`,
   no `<image>`); SHA-256 of each asset; check imports and exports
   against `[api]` (issue 02); write
   `~/.local/share/otty/plugins/<id>/<version>/`; compile into the cache;
   then `plugins.json` (URL, version, asset hashes per tag, the granted
   permissions of each part, each part on or off, the user's choice for
   each conflicting id, removed-by-user for embedded ones), written
   atomically.
6. Update, once a day and on demand: list releases, pick the newest
   whose contract versions are all known (a newer one that is not is
   reported as "needs a newer OTTY"); a tag already recorded whose assets
   now hash differently is refused with the message of the spec; the
   permission diff per part (added, removed, new parts, removed parts)
   between the granted and the new manifest. With no new permission and
   no new part the update installs on its own; otherwise it waits for the
   user and the old version keeps running. Install next to the old,
   switch, then delete the old folder; parts keep their on/off state and
   choices.
7. Turn a part on or off; remove a whole plugin (removes the folder and
   its compiled copy; `agents.json` untouched).
8. Embedded plugins: `assets/plugins/<id>/` built copies with their
   hashes, installed on first start with no dialog and all parts on, not
   reinstalled after the user removed them. All five official plugins
   are embedded (spec Decision 10). While a plugin has no repository of
   its own, a newer copy in a newer OTTY is installed like an update (on
   its own, or waiting when it asks for new permissions); once it has
   one, a newer release replaces it through the normal update. (The
   copies arrive in issues 11, 13, 14 and 16, built from `plugins/<id>/`
   by the script of issue 11; here a test copy of the template.)
9. Local plugins: install from a folder holding `otty-plugin.toml`, the
   built `.wasm` and the icons, with the same checks and conflict
   choices as step 5, the files copied from the folder; `plugins.json`
   records the folder; marked "Local", never "Official", left out of the
   update check; **Reload** copies and checks the folder again, shows the
   permission diff when it asks for more, and switches to the new copy.

## Done when

- Tests first, with a `mockall` mock of the HTTP client and made-up
  release lists: non-semver tags ignored; newest compatible chosen; a
  release with an unknown contract version skipped with the reason; a
  changed asset for a known tag refused; a taken plugin id refused naming
  it; a part conflict is not refused and the losing part is off after
  the choice; "Official" follows the constant; the permission diff lists
  added and removed items and new parts; an update with no new
  permission installs on its own; one with new permissions or a new part
  waits; an embedded plugin installs on first start and stays removed
  after Remove; a newer embedded copy in a newer OTTY is installed like
  an update; a local plugin installs from a folder, is never updated on
  its own, and Reload picks up a rebuilt `.wasm`; `plugins.json` survives
  a crash mid-write.
- The checks of `AGENTS.md` pass.

## Files

`crates/plugins/releases/` (new), `Cargo.toml`, `assets/plugins/`.

## Comments
