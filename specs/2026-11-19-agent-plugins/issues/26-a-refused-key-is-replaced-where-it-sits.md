# 26 A refused key is replaced where it sits

Status: done
Blocked by: 21

## Why

Owner, 2026-11-21: a DeepSeek or OpenRouter account has a key either in
an agent's files (found by OTTY, for example in OpenCode's settings) or
added by hand in OTTY. When the provider refuses the key, the part now
always opens the provider's keys page and says to put the new key where
the old one sits. For a key added by hand in OTTY that place is OTTY
itself, so the need should be a field for the new key. The owner wants
to see how this works in practice.

## What to build

1. A part can tell whether a key was given in OTTY (by hand, or for an
   `ask-key` slot) or read from a place of an agent. Keep it to that one
   fact; the part never sees the key.
2. DeepSeek and OpenRouter: a refused key given in OTTY gives an
   `ask-key` need that replaces that key; a refused key from an agent's
   files keeps the need that opens the keys page. Same for any other
   official part that refuses keys this way.
3. Docs (`Write a Plugin.mdx`) and `spec.md`.

## Done when

- Tests first: both cases give the right need; a new key given in the
  field replaces the old one and the next read uses it.
- The checks of `AGENTS.md` pass.

## Comments

- 2026-11-21: Done. A key handle tells one more fact, `given-in-otty`
  (`Key::is_given_in_otty` in the SDK), and `key-ask.slot` is optional:
  no slot means the account's own key. DeepSeek, OpenRouter and the
  template answer a refused key given in OTTY with `ask-key` and no slot
  (the field opens with the old file or variable and saves the new one in
  its stead, as Edit key… does; the account is read again), and a key an
  agent keeps with the keys page as before. Anthropic and OpenAI ask for
  a refused own admin key added by hand in its place, and take a key given
  under `admin` before the account's own one.
