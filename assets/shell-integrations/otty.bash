# ============================
# OTTY Bash integration hooks
# ============================
#
# Source this file from your interactive bash (e.g. in ~/.bashrc) to emit
# otty-block events for each prompt/command pair.
#
# The script prefers jq/python/perl for JSON escaping and falls back to a
# POSIX-compatible implementation when none are available.

[[ $- != *i* ]] && return 0
[[ -n ${OTTY_BASH_HOOK_INITIALIZED:-} ]] && return 0
OTTY_BASH_HOOK_INITIALIZED=1

otty_block_seq=0
otty_prompt_seq=0
__otty_preexec_fired=0

_otty_json_escape_fallback() {
  local input=$1
  local output='"'
  local len=${#input}
  local i char code

  for ((i = 0; i < len; ++i)); do
    char=${input:i:1}
    case $char in
      '"') output+='\"' ;;
      '\\') output+='\\\\' ;;
      $'\b') output+='\\b' ;;
      $'\f') output+='\\f' ;;
      $'\n') output+='\\n' ;;
      $'\r') output+='\\r' ;;
      $'\t') output+='\\t' ;;
      *)
        if [[ $char =~ [[:cntrl:]] ]]; then
          printf -v code '%d' "'$char"
          printf -v output '%s\\u%04X' "$output" "$code"
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
  printf '\033P'; printf 'otty-block;%s' "$1"; printf '\033\\'
}

_otty_preexec() {
  local id="cmd-$((++otty_block_seq))"
  local cmd_json
  cmd_json=$(_otty_json_escape "$1")
  _otty_emit "{\"v\":1,\"id\":\"$id\",\"phase\":\"preexec\",\"cmd\":$cmd_json}"
}

_otty_precmd() {
  local prompt_id="prompt-$((++otty_prompt_seq))"
  _otty_emit "{\"v\":1,\"id\":\"$prompt_id\",\"phase\":\"precmd\"}"
}

_otty_should_skip_command() {
  local cmd=$1
  [[ -z $cmd ]] && return 0
  case $cmd in
    _otty_*|PROMPT_COMMAND*|trap*|builtin*|local*) return 0 ;;
  esac
  return 1
}

_otty_debug_trap() {
  if (( __otty_preexec_fired )); then
    return
  fi
  local cmd=$BASH_COMMAND
  if ! _otty_should_skip_command "$cmd"; then
    return
  fi
  __otty_preexec_fired=1
  trap - DEBUG
  _otty_preexec "$cmd"
  trap '_otty_debug_trap' DEBUG
}

_otty_precmd_wrapper() {
  __otty_preexec_fired=0
  _otty_precmd
}

if [[ -n ${PROMPT_COMMAND:-} ]]; then
  PROMPT_COMMAND="_otty_precmd_wrapper;${PROMPT_COMMAND}"
else
  PROMPT_COMMAND="_otty_precmd_wrapper"
fi

trap '_otty_debug_trap' DEBUG
