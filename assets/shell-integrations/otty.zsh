# ===========================
# OTTY Zsh integration hooks
# ===========================
#
# Source this file from your interactive zsh (e.g. in ~/.zshrc) to emit
# otty-block events for each prompt/command pair.
#
# The script prefers jq/python/perl for JSON escaping and falls back to a
# POSIX-compatible implementation when none are available.

[[ $- != *i* ]] && return 0
[[ -n ${OTTY_ZSH_HOOK_INITIALIZED:-} ]] && return 0
OTTY_ZSH_HOOK_INITIALIZED=1

autoload -Uz add-zsh-hook

otty_block_seq=0
otty_prompt_seq=0

_otty_json_escape_fallback() {
  local input=$1
  local output='"'
  local len=${#input}
  local i char code hex

  for ((i = 1; i <= len; ++i)); do
    char=${input[i]}
    case $char in
      '"') output+='\"' ;;
      '\\') output+='\\\\' ;;
      $'\b') output+='\\b' ;;
      $'\f') output+='\\f' ;;
      $'\n') output+='\\n' ;;
      $'\r') output+='\\r' ;;
      $'\t') output+='\\t' ;;
      *)
        if [[ $char == [[:cntrl:]] ]]; then
          printf -v code '%d' "'$char"
          printf -v hex '%02X' "$code"
          output+="\\u00$hex"
        else
          output+=$char
        fi
        ;;
    esac
  done

  output+='"'
  printf '%s' "$output"
}

_otty_json_escape() {
  local input=$1

  if command -v jq >/dev/null 2>&1; then
    jq -Rn --arg s "$input" '$s'
    return
  fi

  if command -v python3 >/dev/null 2>&1; then
    python3 -c 'import json, sys; print(json.dumps(sys.argv[1]))' "$input"
    return
  fi

  if command -v python >/dev/null 2>&1; then
    python -c 'import json, sys; print(json.dumps(sys.argv[1]))' "$input"
    return
  fi

  if command -v perl >/dev/null 2>&1; then
    perl -MJSON::PP -we 'print encode_json($ARGV[0])' "$input"
    return
  fi

  _otty_json_escape_fallback "$input"
}

_otty_emit() {
  printf '\033P'; printf 'otty-dcs;block;%s' "$1"; printf '\033\\'
}

_otty_preexec() {
  local id="cmd-$((++otty_block_seq))"
  local cmd_json
  local cwd_json
  local now
  cmd_json=$(_otty_json_escape "$1")
  cwd_json=$(_otty_json_escape "$PWD")
  now=${EPOCHSECONDS:-}
  if [[ -z "$now" ]]; then
    now=$(date +%s 2>/dev/null || echo 0)
  fi
  _otty_emit "{\"v\":1,\"id\":\"$id\",\"phase\":\"preexec\",\"cmd\":$cmd_json,\"cwd\":$cwd_json,\"time\":$now}"
}

_otty_precmd() {
  local prompt_id="prompt-$((++otty_prompt_seq))"
  local cwd_json
  local now
  cwd_json=$(_otty_json_escape "$PWD")
  now=${EPOCHSECONDS:-}
  if [[ -z "$now" ]]; then
    now=$(date +%s 2>/dev/null || echo 0)
  fi
  _otty_emit "{\"v\":1,\"id\":\"$prompt_id\",\"phase\":\"precmd\",\"cwd\":$cwd_json,\"time\":$now}"
}

add-zsh-hook preexec _otty_preexec
add-zsh-hook precmd _otty_precmd
