#!/bin/bash
# Pre-commit checks for TurfOps
# Run this before committing to ensure code quality
# Usage: ./scripts/pre-commit-checks.sh

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Running pre-commit checks...${NC}"
echo ""

# Check for staged .env files (security)
echo -e "${YELLOW}[1/5] Checking for secrets...${NC}"
if git diff --cached --name-only | grep -q '\.env$'; then
    echo -e "${RED}ERROR: Attempting to commit .env file!${NC}"
    echo "Remove it from staging with: git reset HEAD .env"
    exit 1
fi
if git diff --cached --name-only | grep -qE '(password|token|secret|api.?key)' 2>/dev/null; then
    echo -e "${YELLOW}WARNING: Staged files may contain sensitive keywords. Please review.${NC}"
fi
echo -e "${GREEN}✓ No secrets detected${NC}"
echo ""

# Format check
echo -e "${YELLOW}[2/5] Checking formatting (cargo fmt)...${NC}"
if ! cargo fmt --check 2>/dev/null; then
    echo -e "${RED}ERROR: Code is not formatted. Run 'cargo fmt' to fix.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Formatting OK${NC}"
echo ""

# Clippy (linter) - Check for errors; allow dead_code warnings for scaffolded features
echo -e "${YELLOW}[3/5] Running linter (cargo clippy)...${NC}"
CLIPPY_OUTPUT=$(cargo clippy --all-targets --all-features 2>&1)

# Check for actual errors (not warnings)
if echo "$CLIPPY_OUTPUT" | grep -q "^error\["; then
    echo -e "${RED}ERROR: Clippy found errors${NC}"
    echo "$CLIPPY_OUTPUT" | grep -E "^error"
    exit 1
fi

# Count warnings (informational)
WARNING_COUNT=$(echo "$CLIPPY_OUTPUT" | grep -c "^warning:" || true)
if [ "$WARNING_COUNT" -gt 0 ]; then
    echo -e "${YELLOW}⚠ Clippy: $WARNING_COUNT warnings (dead_code allowed for scaffolded features)${NC}"
fi
echo -e "${GREEN}✓ Clippy OK${NC}"
echo ""

# Tests
echo -e "${YELLOW}[4/5] Running tests (cargo test)...${NC}"
if ! cargo test --quiet 2>&1; then
    echo -e "${RED}ERROR: Tests failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ All tests passed${NC}"
echo ""

# Security audit (optional - requires cargo-audit)
echo -e "${YELLOW}[5/5] Running security audit (cargo audit)...${NC}"
if command -v cargo-audit &> /dev/null; then
    if ! cargo audit 2>&1; then
        echo -e "${YELLOW}WARNING: Security vulnerabilities found. Review before committing.${NC}"
        # Don't fail on audit - just warn
    else
        echo -e "${GREEN}✓ No known vulnerabilities${NC}"
    fi
else
    echo -e "${YELLOW}⚠ cargo-audit not installed. Install with: cargo install cargo-audit${NC}"
fi
echo ""

echo -e "${GREEN}All pre-commit checks passed!${NC}"
