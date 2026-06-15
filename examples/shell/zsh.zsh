__snarkterm_now_ms() {
  date +%s%3N 2>/dev/null || python3 -c 'import time; print(int(time.time() * 1000))'
}

__snarkterm_osc() {
  printf '\033]777;snarkterm;%s\007' "$1"
}

preexec() {
  __snarkterm_started_ms="$(__snarkterm_now_ms)"
  __snarkterm_osc "event=command_start;cwd=$PWD;command=$1"
}

precmd() {
  local status="$?"
  local now duration
  now="$(__snarkterm_now_ms)"
  if [[ -n "${__snarkterm_started_ms:-}" ]]; then
    duration=$((now - __snarkterm_started_ms))
    __snarkterm_osc "event=command_end;status=$status;duration_ms=$duration;cwd=$PWD"
    unset __snarkterm_started_ms
  fi
  __snarkterm_osc "event=prompt_start;cwd=$PWD"
}
