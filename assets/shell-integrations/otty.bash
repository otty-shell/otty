# ============================
# OTTY Bash integration hooks
# ============================
#
# Source this file from your interactive bash (e.g. in ~/.bashrc) to emit
# otty-block events for each prompt/command pair.
#
# This script embeds a minimal copy of bash-preexec (MIT-licensed) to
# provide zsh-like preexec/precmd hooks, and uses them to emit OTTY block
# events. See https://github.com/rcaloras/bash-preexec for the original.

[[ $- != *i* ]] && return 0
[[ -n ${OTTY_BASH_HOOK_INITIALIZED:-} ]] && return 0
OTTY_BASH_HOOK_INITIALIZED=1

otty_block_seq=0
otty_prompt_seq=0

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
  printf '\033P'; printf 'otty-dcs;block;%s' "$1"; printf '\033\\'
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

# ---- OTTY-specific preexec/precmd using bash-preexec ----

_otty_preexec() {
  local cmd=${1:-$BASH_COMMAND}
  [[ -z $cmd ]] && return 0
  local id="cmd-$((++otty_block_seq))"
  local cmd_json
  cmd_json=$(_otty_json_escape "$cmd")
  _otty_emit "{\"v\":1,\"id\":\"$id\",\"phase\":\"preexec\",\"cmd\":$cmd_json}"
}

_otty_precmd() {
  local prompt_id="prompt-$((++otty_prompt_seq))"
  _otty_emit "{\"v\":1,\"id\":\"$prompt_id\",\"phase\":\"precmd\"}"
}

preexec_functions+=(_otty_preexec)
precmd_functions+=(_otty_precmd)
