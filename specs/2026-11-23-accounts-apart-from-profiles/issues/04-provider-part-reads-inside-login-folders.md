# 04 A provider part may read logins inside the folders OTTY gives it

Status: done
Blocked by: 03

## Why

Decision 5: a provider part finds accounts in the profiles of the agent
parts of its own plugin, and later in its own hidden profiles
(Decision 6). It reads the login there itself, with its own declared
permissions, and gets keys only as handles. "What OTTY stores" and the
WIT sketch add: the part may read its login files and run its programs
only inside the login folders OTTY passes to the current call, so a
profile the user added in any folder works without a wider glob.

Today a provider part never reads a login: OTTY opens the key an agent
part found and lends the provider part a handle. This issue gives the
host and the manifest what the new contract of issue 05 needs, and uses
it with today's contract, so it is a slice of its own: the folders of a
call are the profiles of `found-in` of today's `login-facts`.

## What to build

1. **The manifest.** A provider part may name the agent parts of the
   same plugin whose profiles it reads, with what it reads inside each
   profile folder. Exact names are free, a shape close to the spec's
   sketch:

   ```toml
   [[agents.provider.logins]]
   agent = "example-agent"           # an agent part of this plugin
   folder_env = "EXAMPLE_AGENT_HOME" # points the agent at a folder
   read = ["sessions/**"]            # inside each folder, read only

   [[agents.provider.logins.keys]]
   files = ["auth.json"]             # inside each folder, keys as handles
   fields = ["/api_key"]
   ```

   `otty-plugin-manifest` (`ProviderPart` in `manifest.rs`, the rules in
   `permissions/declared.rs`) reads and checks it: `agent` is an agent
   part of the same manifest, paths are relative with no `..` and no
   leading `/` or `~`, `fields` are JSON pointers as in the `keys` of an
   agent part.
2. **The host.** The calls of the provider contract
   (`crates/plugins/host/src/agents/provider.rs` `ProviderCalls`) take the
   login folders of the call. For that call only, the permissions of the
   part (`Permissions` of `permissions/check.rs`, built per call in
   `crates/plugins/host/src/calls.rs`) also allow `read` and
   `read-json-with-keys` (keys replaced by handles) of the relative rules
   joined to each folder, and a program the part starts may get
   `folder_env` only with one of those folders as its value. Nothing of
   these rules applies outside a call with folders. In this issue the
   folders of a call are the folders of the profiles in `found-in` of
   the call's `login-facts` whose plugin is the part's plugin and whose
   agent part is one the part names in `logins`.
3. **What the user sees.** The groups of "<Part> can access" and of the
   install and update dialogs (issue 36, `permissions/words.rs`
   `PermissionLine`) show the new rule as a group of its own, for
   example "Logins in Example Agent profiles": the files, "given as
   handles", and the read globs, "read only", each inside the profile's
   folder.
4. **The template.** The template's provider part
   (`plugins/template/src/provider.rs`, `otty-plugin.toml`) declares a
   `logins` entry for `example-agent` and reads the `api_key` handle of
   `auth.json` in the call's folder itself. Rebuild with
   `plugins/build.sh`. The other official plugins move to it in issue 05.
5. Write a Plugin's manifest reference follows in issue 10.

## Done when

- Tests first (manifest): a `logins` entry naming an agent part of
  another plugin or no part, an absolute or `..` path, and a bad pointer
  are refused. (Host, over the template): in a call with a folder the
  part reads `auth.json` there with its key as a handle, also for a
  folder outside every glob of the part, the same read in a call with no
  folder is refused, a file of the folder not declared is refused, and a
  program started with `folder_env` pointing at another folder is
  refused. (Plugins mode): the new group shows in the "can access"
  dialog.
- The checks of `AGENTS.md` pass.

## Comments

2026-11-23, done. A provider part names the profiles it reads with
`[[agents.provider.logins]]`: `agent` (an agent part of the same
manifest), an optional `folder_env`, `read` (globs inside the folder)
and `[[agents.provider.logins.keys]]` with `files` and `fields`.
`otty-plugin-manifest` reads it into `LoginRule` and `LoginKeys`
(`ProviderPart::logins`, also `PartPermissions::logins`). The paths are
a new `FolderGlob`: relative, no leading `/` or `~`, no `..`, no
variable (`GlobProblem::NotInsideFolder` is new). An agent part of
another plugin, a provider part's id or an empty `agent`, a field that
is not a JSON pointer, a key file with no `fields` and a bad variable
name are refused.

`Permissions::in_login_folders(&[LoginFolder])` gives the permissions of
one call: inside each folder of an agent part the part names, it reads
the `read` globs and the key files with keys as handles (also for
`key_fields` and `may_open_key`), and a key file is never read as bytes
or written. `Permissions::may_pass_var(name, value)` replaces
`may_read_var` for the variables of a program or a hidden terminal: in
a call with folders, a `folder_env` may only point at one of them; any
other variable still needs `env`. With no folder nothing of this
applies.

In the host, `CallBatch::call_in_folders` sets the permissions of each
call (files, programs and terminals together), and `call` is the same
with no folder. The three calls of `ProviderCalls` give the folders of
the profiles in `found-in` of their `login-facts` whose plugin is the
part's own; the part's `logins` choose by agent part. The SDK test host
has `TestHost::as_provider_in(id, found, run)`, which gives the folders
the same way, and checks program variables with `may_pass_var`.

Plugins mode shows the new rule as a group of its own per agent part,
"Logins in Example Agent profiles", with the meaning "Only inside the
folder of each profile OTTY gives it", the login files noted "given as
handles" and the read globs "read only", in the Can access dialog and
in the install and update dialogs. The group label is a `String` now
(`PlaceViewModel::profiles_of`). Four new texts in all ten locales.

The template `example` 0.1.2 -> 0.1.3 declares a `logins` entry for
`example-agent` (`EXAMPLE_AGENT_HOME`, `sessions/*.jsonl`, the
`/api_key` of `auth.json`) and its `numbers` reads that key itself in
the folder, else uses the key OTTY lends. It was rebuilt into
`assets/plugins/example/` and copied to
`crates/plugins/host/tests/fixtures/template.wasm`. The template README
says how `logins` works.

Tests: manifest (refusals, words, permissions in and out of folders),
host over the template (the key read in a folder outside every glob,
the same read with no folder, with a folder of another agent part and
of another plugin refused, an undeclared file of the folder refused),
host over the `programs` test plugin (a program with `folder_env`
pointing at another folder, or in a call with no folder, is refused; a
test of the template could not start a program, it runs none), SDK test
host, and Plugins mode (the group in the dialogs).

Left for issue 05: the folders still come from `found-in` of
`login-facts`; issue 05 passes the `login-folder` list instead and
drops `GetsProviderKeys` ("Found logins"). The other official plugins
do not declare `logins` yet. The plain reads (`read`) of a profile
folder that is a symlink to another place are refused, since the
real-path check knows only the home folder and the variable folders, as
for any glob today; key files are read as before.
