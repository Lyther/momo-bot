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

# Run with logs
cmd_run_dev() {
    print_header "Running Momo Bot (Development)"
    check_env

    echo "Starting bot with cargo run..."
    echo ""
    RUST_LOG=info cargo run
}

# Check and build
cmd_full() {
    print_header "Full Check & Build"

    echo "Step 1: Checking code..."
    cargo check
    print_success "Check passed!"

    echo ""
    echo "Step 2: Clean build..."
    cargo clean
    print_success "Cleaned"

    echo ""
    echo "Step 3: Building release..."
    cargo build --release
    print_success "Build complete!"

    echo ""
    print_success "All done! Ready to run."
}

# Run tests
cmd_test() {
    print_header "Running Tests"
    cargo test
}

# Format code
cmd_fmt() {
    print_header "Formatting Code"
    cargo fmt
    print_success "Code formatted!"
}

# Clippy lints
cmd_lint() {
    print_header "Running Clippy"
    cargo clippy -- -D warnings
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
    echo -e "  ${YELLOW}check${NC}       Quick check (cargo check)"
    echo -e "  ${YELLOW}build${NC}       Clean build with no cache (release)"
    echo -e "  ${YELLOW}dev${NC}         Quick development build (debug)"
    echo -e "  ${YELLOW}run${NC}         Run the bot (builds if needed, reads .env)"
    echo -e "  ${YELLOW}run-dev${NC}     Run with cargo run (development mode, reads .env)"
    echo -e "  ${YELLOW}full${NC}        Full check + clean + build pipeline"
    echo -e "  ${YELLOW}test${NC}        Run tests"
    echo -e "  ${YELLOW}fmt${NC}         Format code with rustfmt"
    echo -e "  ${YELLOW}lint${NC}        Run clippy lints"
    echo -e "  ${YELLOW}help${NC}        Show this help message"
    echo ""
    echo -e "${GREEN}Environment Variables Required:${NC}"
    echo "  GROK_API_KEY      Your Grok API key from xAI"
    echo "  TELOXIDE_TOKEN    Your Telegram bot token"
    echo ""
    echo -e "${GREEN}Environment Setup:${NC}"
    echo "  Create a .env file with:"
    echo "    GROK_API_KEY=your-grok-key-here"
    echo "    TELOXIDE_TOKEN=your-telegram-token-here"
    echo "  Or export them manually:"
    echo "    export GROK_API_KEY='your-key'"
    echo "    export TELOXIDE_TOKEN='your-token'"
    echo ""
    echo -e "${GREEN}Examples:${NC}"
    echo "  ./momo.sh check           # Quick validation"
    echo "  ./momo.sh full            # Complete rebuild from scratch"
    echo "  ./momo.sh run             # Start the bot (loads .env)"
    echo "  ./momo.sh run-dev         # Start with detailed logs"
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
        check)
            cmd_check
            ;;
        build)
            cmd_build
            ;;
        dev)
            cmd_dev
            ;;
        run)
            cmd_run
            ;;
        run-dev)
            cmd_run_dev
            ;;
        full)
            cmd_full
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
