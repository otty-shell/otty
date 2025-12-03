autoload -Uz add-zsh-hook
otty_block_seq=0
function _otty_emit() {
  printf '\033P'; printf 'otty-block;%s' "$1"; printf '\033\\'
}
function _otty_preexec() {
  local id="cmd-$((++otty_block_seq))"
  CURRENT_OTTY_BLOCK="$id"
  _otty_emit "{\"v\":1,\"id\":\"$id\",\"phase\":\"preexec\",\"cmd\":\"$1\"}"
}
function _otty_precmd() {
  local prompt_id="prompt-$((++otty_prompt_seq))"
  _otty_emit "{\"v\":1,\"id\":\"$prompt_id\",\"phase\":\"precmd\"}"
}

# function _otty_precmd() {
#   [[ -n "$CURRENT_OTTY_BLOCK" ]] || return
#   _otty_emit "{\"v\":1,\"id\":\"$CURRENT_OTTY_BLOCK\",\"phase\":\"exit\",\"exit_code\":$?}"
#   _otty_emit "{\"v\":1,\"id\":\"prompt-$otty_block_seq\",\"phase\":\"precmd\"}"
#   CURRENT_OTTY_BLOCK=""
# }
add-zsh-hook preexec _otty_preexec
add-zsh-hook precmd _otty_precmd
