# 04 A provider part may read logins inside the folders OTTY gives it

Status: ready
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
