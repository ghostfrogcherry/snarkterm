function __snarkterm_now_ms
    date +%s%3N 2>/dev/null; or python3 -c 'import time; print(int(time.time() * 1000))'
end

function __snarkterm_osc
    printf '\033]777;snarkterm;%s\007' $argv[1]
end

function __snarkterm_command_start --on-event fish_preexec
    set -g __snarkterm_started_ms (__snarkterm_now_ms)
    __snarkterm_osc "event=command_start;cwd=$PWD;command=$argv"
end

function __snarkterm_command_end --on-event fish_postexec
    set status_code $status
    set now (__snarkterm_now_ms)
    if set -q __snarkterm_started_ms
        set duration (math $now - $__snarkterm_started_ms)
        __snarkterm_osc "event=command_end;status=$status_code;duration_ms=$duration;cwd=$PWD"
        set -e __snarkterm_started_ms
    end
    __snarkterm_osc "event=prompt_start;cwd=$PWD"
end
