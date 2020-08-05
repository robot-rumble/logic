# helper "library" for multi-target build scripts

ensure_some_target() {
    declare -n targetvar
    for targetvar in "$@"; do
        [[ $targetvar ]] && return
    done
    echo "No build targets selected, exiting..."
    exit 1
}

ansi_color() {
    local colorcode varname=$1
    shift
    colorcode=$(IFS=';'; echo "$*")
    declare -g "$varname=$(printf '\033[%sm' "$colorcode")"
}
ansi_color BOLD 1
ansi_color NC   0

wait_pids() {
    code=0

    for pid in "$@"; do
        wait "$pid" || code=1
    done

    exit $code
}

prepend() {
    while IFS='' read -r line; do
        echo "$BOLD$1$NC" "$line"
    done
}

# look into this to avoid non-concurrency b/c of cargo file locks?
cargo_build_targeting() {
    local target=$1
    shift
    CARGO_TARGET_DIR=target/"$target" CARGO_BUILD_TARGET=$target cargo build "$@"
}
