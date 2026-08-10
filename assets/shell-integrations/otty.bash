# ============================
# OTTY Bash integration hooks
# ============================
#
# Source this file from an interactive Bash to emit protocol-v2 lifecycle
# events for each prompt/command pair.
#
# This script embeds a minimal copy of bash-preexec (MIT-licensed) to
# provide zsh-like preexec/precmd hooks, and uses them to emit OTTY block
# events. See https://github.com/rcaloras/bash-preexec for the original.

[[ $- != *i* ]] && return 0
[[ -z ${OTTY_TERMINAL_SESSION_ID:-} ]] && return 0
[[ ${OTTY_BASH_HOOK_PID:-} == "$$" ]] && return 0
OTTY_BASH_HOOK_PID=$$

_otty_parent_shell_instance_id=${OTTY_SHELL_INSTANCE_ID:-}
_otty_shell_depth=$((${OTTY_SHELL_DEPTH:--1} + 1))
_otty_shell_instance_id="${OTTY_TERMINAL_SESSION_ID}:bash:$$:${_otty_shell_depth}"
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

_otty_hex_encode() {
  local LC_ALL=C
  local input=$1
  local output=
  local len=${#input}
  local i char code

  for ((i = 0; i < len; ++i)); do
    char=${input:i:1}
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
  shell_json=$(_otty_json_escape "bash")
  version_json=$(_otty_json_escape "$BASH_VERSION")
  if [[ -n $_otty_parent_shell_instance_id ]]; then
    parent_json=$(_otty_json_escape "$_otty_parent_shell_instance_id")
  else
    parent_json=null
  fi
  _otty_emit_event "shell_hello" "" "{\"shell\":$shell_json,\"shell_version\":$version_json,\"parent_shell_instance_id\":$parent_json,\"capabilities\":[\"command_end\",\"osc_133\",\"nested_shell\"]}"
}

# ---- Minimal bash-preexec core (MIT) ----

if [[ -z "${BASH_VERSION:-}" ]]; then
  return 0
fi

if [[ -n "${bash_preexec_imported:-}" ]]; then
  :
else
  bash_preexec_imported="defined"
  __bp_last_ret_value="$?"
  BP_PIPESTATUS=("${PIPESTATUS[@]}")
  __bp_last_argument_prev_command="$_"

  __bp_inside_precmd=0
  __bp_inside_preexec=0

  __bp_install_string=$'__bp_trap_string="$(trap -p DEBUG)"\ntrap - DEBUG\n__bp_install'

  __bp_trim_whitespace() {
    local var=${1:?} text=${2:-}
    text="${text#"${text%%[![:space:]]*}"}"
    text="${text%"${text##*[![:space:]]}"}"
    printf -v "$var" '%s' "$text"
  }

  __bp_sanitize_string() {
    local var=${1:?} text=${2:-} sanitized
    __bp_trim_whitespace sanitized "$text"
    sanitized=${sanitized%;}
    sanitized=${sanitized#;}
    __bp_trim_whitespace sanitized "$sanitized"
    printf -v "$var" '%s' "$sanitized"
  }

  __bp_set_ret_value() {
    return ${1:-}
  }

  __bp_in_prompt_command() {
    local prompt_command_array
    IFS=$'\n;' read -rd '' -a prompt_command_array <<< "${PROMPT_COMMAND:-}"

    local trimmed_arg
    __bp_trim_whitespace trimmed_arg "${1:-}"

    local command trimmed_command
    for command in "${prompt_command_array[@]:-}"; do
      __bp_trim_whitespace trimmed_command "$command"
      if [[ "$trimmed_command" == "$trimmed_arg" ]]; then
        return 0
      fi
    done

    return 1
  }

  __bp_precmd_invoke_cmd() {
    __bp_last_ret_value="$?" BP_PIPESTATUS=("${PIPESTATUS[@]}")

    if (( __bp_inside_precmd > 0 )); then
      (exit $__bp_last_ret_value)
      return
    fi
    local __bp_inside_precmd=1

    local precmd_function
    for precmd_function in "${precmd_functions[@]:-}"; do
      if type -t "$precmd_function" >/dev/null 2>&1; then
        __bp_set_ret_value "$__bp_last_ret_value" "$__bp_last_argument_prev_command"
        "$precmd_function"
      fi
    done
    (exit $__bp_last_ret_value)
  }

  __bp_preexec_interactive_mode=""

  __bp_interactive_mode() {
    __bp_preexec_interactive_mode="on"
  }

  __bp_preexec_invoke_exec() {
    __bp_last_argument_prev_command="${1:-}"
    if (( __bp_inside_preexec > 0 )); then
      return
    fi
    local __bp_inside_preexec=1

    if [[ ! -t 1 && -z "${__bp_delay_install:-}" ]]; then
      return
    fi

    if [[ -n "${COMP_LINE:-}" ]]; then
      return
    fi
    if [[ -z "${__bp_preexec_interactive_mode:-}" ]]; then
      return
    else
      if [[ 0 -eq "${BASH_SUBSHELL:-}" ]]; then
        __bp_preexec_interactive_mode=""
      fi
    fi

    if __bp_in_prompt_command "${BASH_COMMAND:-}"; then
      __bp_preexec_interactive_mode=""
      return
    fi

    local this_command
    this_command=$(
      HISTTIMEFORMAT= builtin history 1 | sed '1 s/^ *[0-9][0-9]*[* ] //'
    )

    if [[ -z "$this_command" ]]; then
      return
    fi

    local preexec_function
    for preexec_function in "${preexec_functions[@]:-}"; do
      if type -t "$preexec_function" >/dev/null 2>&1; then
        __bp_set_ret_value ${__bp_last_ret_value:-}
        "$preexec_function" "$this_command"
      fi
    done
  }

  __bp_install() {
    if [[ "${PROMPT_COMMAND:-}" == *"__bp_precmd_invoke_cmd"* ]]; then
      return 0
    fi

    trap '__bp_preexec_invoke_exec "$_"' DEBUG

    local existing_prompt_command
    existing_prompt_command="${PROMPT_COMMAND:-}"
    existing_prompt_command="${existing_prompt_command//$__bp_install_string[;$'\n']}"
    existing_prompt_command="${existing_prompt_command//$__bp_install_string}"
    __bp_sanitize_string existing_prompt_command "$existing_prompt_command"

    PROMPT_COMMAND=$'__bp_precmd_invoke_cmd\n'
    if [[ -n "$existing_prompt_command" ]]; then
      PROMPT_COMMAND+="${existing_prompt_command}"$'\n'
    fi
    PROMPT_COMMAND+='__bp_interactive_mode'

    precmd_functions+=(precmd)
    preexec_functions+=(preexec)

    __bp_precmd_invoke_cmd
    __bp_interactive_mode
  }

  __bp_install_after_session_init() {
    local sanitized_prompt_command
    __bp_sanitize_string sanitized_prompt_command "${PROMPT_COMMAND:-}"
    if [[ -n "$sanitized_prompt_command" ]]; then
      PROMPT_COMMAND="${sanitized_prompt_command}"$'\n'
    fi
    PROMPT_COMMAND+="${__bp_install_string}"
  }

  declare -a precmd_functions
  declare -a preexec_functions

  __bp_install_after_session_init
fi

# ---- OTTY-specific protocol-v2 lifecycle using bash-preexec ----

_otty_preexec() {
  local cmd=${1:-$BASH_COMMAND}
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
  local status=$?
  local pipeline_status=("${BP_PIPESTATUS[@]:-$status}")
  local cwd_json pipe_json separator value

  cwd_json=$(_otty_json_escape "$PWD")
  if [[ -n $_otty_active_block_id ]]; then
    pipe_json='['
    separator=
    for value in "${pipeline_status[@]}"; do
      pipe_json+="${separator}${value}"
      separator=,
    done
    pipe_json+=']'
    _otty_emit_event "command_end" "$_otty_active_block_id" "{\"exit_code\":$status,\"pipe_status\":$pipe_json,\"cwd\":$cwd_json}"
    _otty_active_block_id=
  fi

  _otty_block_seq=$((_otty_block_seq + 1))
  _otty_prepared_block_id="${OTTY_TERMINAL_SESSION_ID}:${_otty_shell_instance_id}:${_otty_block_seq}"
  _otty_emit_event "prompt_prepare" "$_otty_prepared_block_id" "{\"cwd\":$cwd_json}"

  return "$status"
}

_otty_shell_exit() {
  local status=$?
  local cwd_json
  cwd_json=$(_otty_json_escape "$PWD")

  if [[ -n $_otty_active_block_id ]]; then
    _otty_emit_event "command_end" "$_otty_active_block_id" "{\"exit_code\":$status,\"pipe_status\":[$status],\"cwd\":$cwd_json}"
    _otty_active_block_id=
  fi
  _otty_emit_event "shell_exit" "" "{\"status\":$status}"

  trap - EXIT
  if [[ -n ${_otty_previous_exit_handler:-} ]]; then
    eval -- "$_otty_previous_exit_handler"
  fi
  return "$status"
}

_otty_install_prompt_markers() {
  PS1='\[\e]133;A\e\\\]'"${PS1:-}"'\[\e]133;B\e\\\]'
}

preexec_functions+=(_otty_preexec)
precmd_functions+=(_otty_precmd)

_otty_previous_exit_handler=$(trap -p EXIT)
_otty_previous_exit_handler=${_otty_previous_exit_handler#trap -- \'}
_otty_previous_exit_handler=${_otty_previous_exit_handler%\' EXIT}
trap '_otty_shell_exit' EXIT

_otty_install_prompt_markers
_otty_emit_shell_hello
