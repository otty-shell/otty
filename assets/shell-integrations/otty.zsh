# ===========================
# OTTY Zsh integration hooks
# ===========================
#
# Source this file from an interactive Zsh to emit protocol-v2 lifecycle
# events. Encoding uses shell builtins only.

[[ $- != *i* ]] && return 0
[[ -z ${OTTY_TERMINAL_SESSION_ID:-} ]] && return 0
[[ ${OTTY_ZSH_HOOK_PID:-} == "$$" ]] && return 0
OTTY_ZSH_HOOK_PID=$$

autoload -Uz add-zsh-hook

_otty_parent_shell_instance_id=${OTTY_SHELL_INSTANCE_ID:-}
_otty_shell_depth=$((${OTTY_SHELL_DEPTH:--1} + 1))
_otty_shell_instance_id="${OTTY_TERMINAL_SESSION_ID}:zsh:$$:${_otty_shell_depth}"
OTTY_SHELL_INSTANCE_ID=$_otty_shell_instance_id
OTTY_SHELL_DEPTH=$_otty_shell_depth
export OTTY_SHELL_INSTANCE_ID OTTY_SHELL_DEPTH

_otty_event_seq=0
_otty_block_seq=0
_otty_prepared_block_id=
_otty_active_block_id=

_otty_json_escape() {
  local LC_ALL=C
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
          printf -v hex '%04X' "$code"
          output+="\\u$hex"
        else
          output+=$char
        fi
        ;;
    esac
  done

  output+='"'
  printf '%s' "$output"
}

_otty_hex_encode() {
  local LC_ALL=C
  local input=$1
  local output=
  local len=${#input}
  local i char code

  for ((i = 1; i <= len; ++i)); do
    char=${input[i]}
    printf -v code '%d' "'$char"
    printf -v output '%s%02X' "$output" "$code"
  done

  printf '%s' "$output"
}

_otty_emit_event() {
  local event=$1
  local block_id=${2:-}
  local payload=$3
  local session_json shell_json block_json block_field json encoded

  session_json=$(_otty_json_escape "$OTTY_TERMINAL_SESSION_ID")
  shell_json=$(_otty_json_escape "$_otty_shell_instance_id")
  block_field=
  if [[ -n $block_id ]]; then
    block_json=$(_otty_json_escape "$block_id")
    block_field=",\"block_id\":$block_json"
  fi
  _otty_event_seq=$((_otty_event_seq + 1))
  json="{\"v\":2,\"event\":\"$event\",\"terminal_session_id\":$session_json,\"shell_instance_id\":$shell_json,\"seq\":$_otty_event_seq${block_field},\"payload\":$payload}"
  encoded=$(_otty_hex_encode "$json")
  printf '\033Potty-dcs;event-v2;h;%s\033\\' "$encoded"
}

_otty_emit_shell_hello() {
  local shell_json version_json parent_json
  shell_json=$(_otty_json_escape "zsh")
  version_json=$(_otty_json_escape "$ZSH_VERSION")
  if [[ -n $_otty_parent_shell_instance_id ]]; then
    parent_json=$(_otty_json_escape "$_otty_parent_shell_instance_id")
  else
    parent_json=null
  fi
  _otty_emit_event "shell_hello" "" "{\"shell\":$shell_json,\"shell_version\":$version_json,\"parent_shell_instance_id\":$parent_json,\"capabilities\":[\"command_end\",\"osc_133\",\"nested_shell\"]}"
}

_otty_preexec() {
  local cmd=$1
  [[ -z $cmd ]] && return 0

  if [[ -z $_otty_prepared_block_id ]]; then
    _otty_block_seq=$((_otty_block_seq + 1))
    _otty_prepared_block_id="${OTTY_TERMINAL_SESSION_ID}:${_otty_shell_instance_id}:${_otty_block_seq}"
  fi
  _otty_active_block_id=$_otty_prepared_block_id

  local cmd_json cwd_json
  cmd_json=$(_otty_json_escape "$cmd")
  cwd_json=$(_otty_json_escape "$PWD")
  _otty_emit_event "command_start" "$_otty_active_block_id" "{\"command\":$cmd_json,\"cwd\":$cwd_json,\"command_truncated\":false}"
}

_otty_precmd() {
  _otty_saved_status=("$?" "${pipestatus[@]}")
  local exit_status=${_otty_saved_status[1]}
  local -a pipeline_status=("${_otty_saved_status[2,-1]}")
  local cwd_json pipe_json separator value

  if (( ${#pipeline_status[@]} == 0 )); then
    pipeline_status=("$exit_status")
  fi

  cwd_json=$(_otty_json_escape "$PWD")
  if [[ -n $_otty_active_block_id ]]; then
    pipe_json='['
    separator=
    for value in "${pipeline_status[@]}"; do
      pipe_json+="${separator}${value}"
      separator=,
    done
    pipe_json+=']'
    _otty_emit_event "command_end" "$_otty_active_block_id" "{\"exit_code\":$exit_status,\"pipe_status\":$pipe_json,\"cwd\":$cwd_json}"
    _otty_active_block_id=
  fi

  _otty_block_seq=$((_otty_block_seq + 1))
  _otty_prepared_block_id="${OTTY_TERMINAL_SESSION_ID}:${_otty_shell_instance_id}:${_otty_block_seq}"
  _otty_emit_event "prompt_prepare" "$_otty_prepared_block_id" "{\"cwd\":$cwd_json}"

  return "$exit_status"
}

_otty_shell_exit() {
  local exit_status=$?
  local cwd_json
  cwd_json=$(_otty_json_escape "$PWD")

  if [[ -n $_otty_active_block_id ]]; then
    _otty_emit_event "command_end" "$_otty_active_block_id" "{\"exit_code\":$exit_status,\"pipe_status\":[$exit_status],\"cwd\":$cwd_json}"
    _otty_active_block_id=
  fi
  _otty_emit_event "shell_exit" "" "{\"status\":$exit_status}"

  return "$exit_status"
}

_otty_install_prompt_markers() {
  PROMPT=$'%{\033]133;A\033\\%}'"${PROMPT:-}"$'%{\033]133;B\033\\%}'
}

add-zsh-hook preexec _otty_preexec
add-zsh-hook precmd _otty_precmd
add-zsh-hook zshexit _otty_shell_exit

_otty_install_prompt_markers
_otty_emit_shell_hello
