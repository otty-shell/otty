autoload -Uz add-zsh-hook
otty_block_seq=0
otty_prompt_seq=0

function _otty_json_escape_fallback() {
  local input="$1"
  local output='"'
  local len=${#input}
  local char code hex

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

function _otty_json_escape() {
  local input="$1"

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

function _otty_emit() {
  printf '\033P'; printf 'otty-block;%s' "$1"; printf '\033\\'
}

function _otty_preexec() {
  local id="cmd-$((++otty_block_seq))"
  local cmd=$(_otty_json_escape "$1")
  _otty_emit "{\"v\":1,\"id\":\"$id\",\"phase\":\"preexec\",\"cmd\":$cmd}"
}

function _otty_precmd() {
  local prompt_id="prompt-$((++otty_prompt_seq))"
  _otty_emit "{\"v\":1,\"id\":\"$prompt_id\",\"phase\":\"precmd\"}"
}

add-zsh-hook preexec _otty_preexec
add-zsh-hook precmd _otty_precmd
