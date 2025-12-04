# ==============================
# OTTY Nushell integration hook
# ==============================
#
# Source from config.nu to register pre_prompt and pre_execution hooks that
# emit otty-block events.

def --env otty_emit [payload: string] {
    let esc = (char escape)
    print -n $"($esc)P"
    print -n $"otty-block;$payload"
    print -n $"($esc)\\"
}

def --env otty_preexec [cmd: string] {
    let seq = 1 + ($env.OTTY_BLOCK_SEQ? | default 0)
    let-env OTTY_BLOCK_SEQ = $seq
    let payload = {
        v: 1,
        id: $"cmd-($seq)",
        phase: "preexec",
        cmd: $cmd
    } | to json -r
    otty_emit $payload
}

def --env otty_precmd [] {
    let seq = 1 + ($env.OTTY_PROMPT_SEQ? | default 0)
    let-env OTTY_PROMPT_SEQ = $seq
    let payload = {
        v: 1,
        id: $"prompt-($seq)",
        phase: "precmd"
    } | to json -r
    otty_emit $payload
}

export-env {
    if not $nu.is-interactive {
        return
    }

    if ($env.OTTY_NU_HOOK_INITIALIZED? | default false) {
        return
    }

    let pre_prompt_hooks = ($env.config.hooks.pre_prompt? | default [])
    let pre_exec_hooks = ($env.config.hooks.pre_execution? | default [])

    let updated_config = (
        $env.config
        | upsert hooks.pre_prompt ($pre_prompt_hooks | append {|| otty_precmd })
        | upsert hooks.pre_execution ($pre_exec_hooks | append {|context| otty_preexec $context.command })
    )

    let-env config = $updated_config
    let-env OTTY_NU_HOOK_INITIALIZED true
}
