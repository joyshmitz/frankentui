#!/usr/bin/env bash
#
# pull_latest_reference_library_repos.sh
#
# Clones or pulls the latest default branch of the three reference libraries
# that FrankenTUI is synthesized from. These are kept in legacy_reference_library_code/
# for reference during development.
#
# Usage:
#   ./scripts/pull_latest_reference_library_repos.sh
#
# This script is idempotent: it clones if the repo doesn't exist, otherwise pulls.
# Called automatically as part of the build process.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
REFERENCE_DIR="$PROJECT_ROOT/legacy_reference_library_code"

# Colors for output (if terminal supports it)
if [[ -t 1 ]]; then
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    RED='\033[0;31m'
    NC='\033[0m' # No Color
else
    GREEN=''
    YELLOW=''
    RED=''
    NC=''
fi

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

# Sync a single repo
# Usage: sync_repo name url branch
# Returns: 0 on success, 1 on failure
sync_repo() {
    local repo_name="$1"
    local repo_url="$2"
    local default_branch="$3"
    local repo_path="$REFERENCE_DIR/$repo_name"

    if [[ -d "$repo_path/.git" ]]; then
        # Repo exists, pull latest
        log_info "Updating $repo_name ($default_branch)..."
        # Note: We use reset --hard to ensure we always match origin exactly.
        # These are read-only reference copies; local modifications are not expected.
        if (cd "$repo_path" && git fetch origin "$default_branch" --quiet && git reset --hard "origin/$default_branch" --quiet); then
            log_info "  $repo_name updated successfully"
            return 0
        else
            log_warn "  Failed to update $repo_name (check network connectivity)"
            return 1
        fi
    else
        # Repo doesn't exist, clone it
        log_info "Cloning $repo_name ($default_branch)..."
        if git clone --depth 1 --branch "$default_branch" "$repo_url" "$repo_path"; then
            log_info "  $repo_name cloned successfully"
            return 0
        else
            log_error "  Failed to clone $repo_name (check network connectivity)"
            return 1
        fi
    fi
}

# Create the reference directory if it doesn't exist
mkdir -p "$REFERENCE_DIR"

# Track results
successes=0
failures=0

# The three reference libraries that FrankenTUI synthesizes from
sync_repo "rich_rust" "https://github.com/Dicklesworthstone/rich_rust.git" "master" && successes=$((successes + 1)) || failures=$((failures + 1))
sync_repo "charmed_rust" "https://github.com/Dicklesworthstone/charmed_rust.git" "master" && successes=$((successes + 1)) || failures=$((failures + 1))
sync_repo "opentui_rust" "https://github.com/Dicklesworthstone/opentui_rust.git" "main" && successes=$((successes + 1)) || failures=$((failures + 1))

# Report results
if [[ $failures -eq 0 ]]; then
    log_info "All reference libraries synchronized successfully ($successes repos)"
elif [[ $successes -eq 0 ]]; then
    log_error "All sync operations failed ($failures repos)"
    exit 1
else
    log_warn "Partial sync: $successes succeeded, $failures failed"
fi

# Print summary
echo ""
echo "Reference library directory: $REFERENCE_DIR"
echo "Contents:"
for repo_name in rich_rust charmed_rust opentui_rust; do
    repo_path="$REFERENCE_DIR/$repo_name"
    if [[ -d "$repo_path" ]]; then
        commit=$(cd "$repo_path" && git rev-parse --short HEAD 2>/dev/null || echo "unknown")
        echo "  $repo_name @ $commit"
    else
        echo "  $repo_name (not present)"
    fi
done
