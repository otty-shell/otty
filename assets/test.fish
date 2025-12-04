# =============================
# OTTY Fish integration hooks
# =============================
#
# Source this file from config.fish (only in interactive shells).

if not status --is-interactive
    return
end

if set -q OTTY_FISH_HOOK_INITIALIZED
    return
end
set -g OTTY_FISH_HOOK_INITIALIZED 1
set -g otty_block_seq 0
set -g otty_prompt_seq 0

function __otty_json_escape_basic --argument input
    set escaped "$input"
    set escaped (string replace -a -- '\\' '\\\\' -- "$escaped")
    set escaped (string replace -a -- '"' '\\"' -- "$escaped")
    set -l ctrl_b (printf '\x08')
    set -l ctrl_f (printf '\x0C')
    set -l ctrl_n (printf '\x0A')
    set -l ctrl_r (printf '\x0D')
    set -l ctrl_t (printf '\x09')
    set escaped (string replace -a -- $ctrl_b '\\b' -- "$escaped")
    set escaped (string replace -a -- $ctrl_f '\\f' -- "$escaped")
    set escaped (string replace -a -- $ctrl_n '\\n' -- "$escaped")
    set escaped (string replace -a -- $ctrl_r '\\r' -- "$escaped")
    set escaped (string replace -a -- $ctrl_t '\\t' -- "$escaped")
    printf '"%s"' "$escaped"
end

function __otty_json_escape --argument input
    if command -sq jq
        jq -Rn --arg s "$input" '$s'
        return
    end

    if command -sq python3
        python3 -c 'import json, sys; print(json.dumps(sys.argv[1]))' "$input"
        return
    end

    if command -sq python
        python -c 'import json, sys; print(json.dumps(sys.argv[1]))' "$input"
        return
    end

    if command -sq perl
        perl -MJSON::PP -we 'print encode_json($ARGV[0])' "$input"
        return
    end

    if functions -q string
        string escape --style json -- "$input" 2>/dev/null
        if test $status -eq 0
            return
        end
    end

    __otty_json_escape_basic "$input"
end

function __otty_emit --argument payload
    printf '\033P'; printf 'otty-block;%s' "$payload"; printf '\033\\'
end

function __otty_fish_preexec --on-event fish_preexec
    set cmd (string join " " -- $argv)
    set seq (math "$otty_block_seq + 1")
    set -g otty_block_seq $seq
    set id "cmd-$seq"
    set escaped (__otty_json_escape "$cmd")
    set payload (string join "" "{\"v\":1,\"id\":\"" $id "\",\"phase\":\"preexec\",\"cmd\":" $escaped "}")
    __otty_emit $payload
end

function __otty_fish_precmd --on-event fish_prompt
    set seq (math "$otty_prompt_seq + 1")
    set -g otty_prompt_seq $seq
    set id "prompt-$seq"
    set payload (string join "" "{\"v\":1,\"id\":\"" $id "\",\"phase\":\"precmd\"}")
    __otty_emit $payload
end
