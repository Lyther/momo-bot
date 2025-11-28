#!/bin/bash
# Momo Bot Management Script
# Usage: ./momo.sh [command]

set -e  # Exit on error

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PID_FILE="$SCRIPT_DIR/momo.pid"
LOG_FILE="$SCRIPT_DIR/momo.log"

join_by() {
    local IFS="$1"; shift; echo "$*"
}

read_pidfile() {
    if [ -f "$PID_FILE" ]; then
        tr -cd '0-9\n ' < "$PID_FILE" | head -n1 | xargs
    fi
}

running_pids() {
    local pids=()
    local pid_from_file
    pid_from_file=$(read_pidfile)
    if [ -n "$pid_from_file" ] && kill -0 "$pid_from_file" >/dev/null 2>&1; then
        pids+=("$pid_from_file")
    fi

    # Prefer exact process name match to skip wrapper shells
    while IFS= read -r p; do
        if [ -n "$p" ] && kill -0 "$p" >/dev/null 2>&1; then
            if [[ ! " ${pids[*]} " =~ " ${p} " ]]; then
                pids+=("$p")
            fi
        fi
    done < <(pgrep -x momo-bot 2>/dev/null || true)

    # Legacy fallback only if none found yet
    if [ ${#pids[@]} -eq 0 ]; then
        while IFS= read -r p; do
            if [ -n "$p" ] && kill -0 "$p" >/dev/null 2>&1; then
                if [[ ! " ${pids[*]} " =~ " ${p} " ]]; then
                    pids+=("$p")
                fi
            fi
        done < <(pgrep -f "target/release/momo-bot" 2>/dev/null || true)
    fi

    echo "${pids[@]}"
}

print_header() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  😼 Momo Bot v2.0 - $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Load environment variables from .env file if it exists
load_env() {
    if [ -f ".env" ]; then
        print_success "Loading .env file"
        set -a  # automatically export all variables
        source .env
        set +a
    fi
}

# Check if environment variables are set
check_env() {
    print_header "Checking Environment"

    # Load .env first
    load_env

    if [ -z "$GROK_API_KEY" ]; then
        print_error "GROK_API_KEY not set"
        echo "Set it in .env file or export: export GROK_API_KEY='your-key'"
        exit 1
    else
        print_success "GROK_API_KEY is set"
    fi

    if [ -z "$TELOXIDE_TOKEN" ]; then
        print_error "TELOXIDE_TOKEN not set"
        echo "Set it in .env file or export: export TELOXIDE_TOKEN='your-token'"
        exit 1
    else
        print_success "TELOXIDE_TOKEN is set"
    fi

    echo ""
}

# Quick check (no clean)
cmd_check() {
    print_header "Quick Check"
    echo "Running cargo check..."
    cargo check
    print_success "Check passed!"
}

# Clean build (no cache)
cmd_build() {
    print_header "Clean Build (No Cache)"
    echo "Cleaning previous build..."
    cargo clean
    print_success "Cleaned build artifacts"

    echo ""
    echo "Building release version..."
    cargo build --release
    print_success "Build complete!"

    echo ""
    echo "Binary location: target/release/momo-bot"
}

# Development build (faster)
cmd_dev() {
    print_header "Development Build"
    echo "Building debug version..."
    cargo build
    print_success "Debug build complete!"

    echo ""
    echo "Binary location: target/debug/momo-bot"
}

# Run the bot
cmd_run() {
    print_header "Running Momo Bot"
    check_env

    if [ ! -f "target/release/momo-bot" ]; then
        print_warning "Release binary not found. Building first..."
        cmd_build
    fi

    echo "Starting bot..."
    echo ""
    ./target/release/momo-bot
}

# Run the bot in the background with logs and PID tracking
cmd_start() {
    print_header "Starting Momo Bot (background)"
    check_env

    if [ ! -f "target/release/momo-bot" ]; then
        print_warning "Release binary not found. Building first..."
        cmd_build
    fi

    local pids
    pids=$(running_pids)
    if [ -n "$pids" ]; then
        print_warning "Bot already running with PID(s): $pids. Use restart or stop."
        exit 0
    fi

    echo "Launching..."
    nohup ./target/release/momo-bot >> "$LOG_FILE" 2>&1 &
    echo $! > "$PID_FILE"
    print_success "Started (PID $(cat "$PID_FILE")). Logs: $LOG_FILE"
}

cmd_stop() {
    print_header "Stopping Momo Bot"
    local pids
    pids=$(running_pids)
    if [ -z "$pids" ]; then
        print_warning "Bot not running."
        rm -f "$PID_FILE"
        return
    fi

    # shellcheck disable=SC2086
    kill $pids
    rm -f "$PID_FILE"
    print_success "Bot stopped (PID $(join_by ' ' $pids))."
}

cmd_restart() {
    print_header "Restarting Momo Bot"
    cmd_stop
    cmd_start
}

cmd_status() {
    local pids
    pids=$(running_pids)
    if [ -n "$pids" ]; then
        print_success "Bot running (PID $(join_by ' ' $pids))."
    else
        print_warning "Bot is not running."
    fi
}

cmd_logs() {
    print_header "Latest Logs"
    if [ -f "$LOG_FILE" ]; then
        tail -n 80 "$LOG_FILE"
    else
        print_warning "Log file not found at $LOG_FILE"
    fi
}

# Show help
cmd_help() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "  ${BLUE}😼 Momo Bot v2.0 - Management Script${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "${GREEN}Usage:${NC}"
    echo "  ./momo.sh [command]"
    echo ""
    echo -e "${GREEN}Commands:${NC}"
    echo -e "  ${YELLOW}build${NC}       Clean build with no cache (release)"
    echo -e "  ${YELLOW}run${NC}         Run the bot in the foreground (reads .env)"
    echo -e "  ${YELLOW}start${NC}       Start in background with logs & PID"
    echo -e "  ${YELLOW}stop${NC}        Stop background bot"
    echo -e "  ${YELLOW}restart${NC}     Restart background bot"
    echo -e "  ${YELLOW}status${NC}      Show running status"
    echo -e "  ${YELLOW}logs${NC}        Tail the latest logs"
    echo -e "  ${YELLOW}check${NC}       Quick check (cargo check)"
    echo ""
    echo -e "${GREEN}Developer helpers:${NC}"
    echo -e "  ${YELLOW}test${NC}        Run tests"
    echo -e "  ${YELLOW}fmt${NC}         Format code with rustfmt"
    echo -e "  ${YELLOW}lint${NC}        Run clippy lints"
    echo -e "  ${YELLOW}help${NC}        Show this help message"
    echo ""
    echo -e "${GREEN}Environment Variables Required:${NC}"
    echo "  GROK_API_KEY      Your Grok API key from xAI"
    echo "  TELOXIDE_TOKEN    Your Telegram bot token"
    echo ""
    echo -e "${GREEN}Optional Model Configuration:${NC}"
    echo "  GROK_MODEL              Chat model (default: grok-4-fast-reasoning)"
    echo "  GROK_MODEL_PROACTIVE    Proactive model (default: grok-4-fast-reasoning)"
    echo "  GROK_MAX_TOKENS         Max tokens per request (default: 2048)"
    echo "  GROK_ENABLE_SEARCH      Enable web search (default: true)"
    echo "  GROK_ENABLE_MULTIMODAL  Enable image processing (default: true)"
    echo ""
    echo -e "${GREEN}Environment Setup:${NC}"
    echo "  Create a .env file with:"
    echo "    GROK_API_KEY=your-grok-key-here"
    echo "    TELOXIDE_TOKEN=your-telegram-token-here"
    echo "    GROK_MODEL=grok-4-1-fast-reasoning"
    echo "    GROK_MODEL_PROACTIVE=grok-4-1-fast-reasoning"
    echo "  Or export them manually:"
    echo "    export GROK_API_KEY='your-key'"
    echo "    export TELOXIDE_TOKEN='your-token'"
    echo ""
    echo -e "${GREEN}Examples:${NC}"
    echo "  ./momo.sh build           # Clean release build"
    echo "  ./momo.sh start           # Start in background (uses .env, writes logs)"
    echo "  ./momo.sh status          # Show running status"
    echo "  ./momo.sh logs            # Tail recent logs"
    echo "  ./momo.sh restart         # Restart background bot"
    echo ""
    echo -e "${GREEN}Configuration:${NC}"
    echo "  Edit src/config.rs to tune behavior rates and timings"
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# Main script logic
main() {
    if [ $# -eq 0 ]; then
        cmd_help
        exit 0
    fi

    case "$1" in
        build)
            cmd_build
            ;;
        run)
            cmd_run
            ;;
        start)
            cmd_start
            ;;
        stop)
            cmd_stop
            ;;
        restart)
            cmd_restart
            ;;
        status)
            cmd_status
            ;;
        check)
            cmd_check
            ;;
        test)
            cmd_test
            ;;
        fmt)
            cmd_fmt
            ;;
        lint)
            cmd_lint
            ;;
        logs)
            cmd_logs
            ;;
        help|--help|-h)
            cmd_help
            ;;
        *)
            print_error "Unknown command: $1"
            echo ""
            cmd_help
            exit 1
            ;;
    esac
}

main "$@"
