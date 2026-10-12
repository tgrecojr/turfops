#!/usr/bin/env bash
# @spec DEPLOY-CI-008
# GitHub settings that keep every change to main going through a pull request.
#
#   scripts/github-protection.sh [check|apply]     (default: check)
#
# check  compares the live settings with the ones below and exits 1 on any difference.
# apply  writes them, then checks. Needs `gh` logged in with admin rights on the repo,
#        so it runs by hand, not in CI.
set -euo pipefail

REPO="tgrecojr/turfops"

# The whole protection on main. A PUT replaces it, so every field is stated:
# - a PR is required, with no approvals (the owner is the only reviewer);
# - GitGuardian stays a required, up-to-date check (a leaked secret cannot be undone);
# - the owner keeps an emergency path (enforce_admins off) and CI jobs are not required
#   checks: policy carries what enforcement does not (deployment design, Decisions);
# - force-pushes and deleting main stay off.
PROTECTION='{
  "required_status_checks": {
    "strict": true,
    "checks": [{ "context": "GitGuardian Security Checks", "app_id": 46505 }]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 0,
    "dismiss_stale_reviews": false,
    "require_code_owner_reviews": false,
    "require_last_push_approval": false
  },
  "restrictions": null,
  "required_linear_history": false,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false,
  "required_conversation_resolution": false,
  "lock_branch": false,
  "allow_fork_syncing": false
}'

# Repository settings: merged PR branches are deleted.
REPO_SETTINGS='{ "delete_branch_on_merge": true }'

# The live settings in the same shape as the desired ones, keys sorted.
live() {
  local protection repo
  protection=$(gh api "repos/$REPO/branches/main/protection" --jq '{
    required_status_checks: {
      strict: .required_status_checks.strict,
      checks: [.required_status_checks.checks[] | {context, app_id}]
    },
    enforce_admins: .enforce_admins.enabled,
    required_pull_request_reviews: (.required_pull_request_reviews | {
      required_approving_review_count, dismiss_stale_reviews,
      require_code_owner_reviews, require_last_push_approval
    }),
    restrictions: (.restrictions // null),
    required_linear_history: .required_linear_history.enabled,
    allow_force_pushes: .allow_force_pushes.enabled,
    allow_deletions: .allow_deletions.enabled,
    block_creations: .block_creations.enabled,
    required_conversation_resolution: .required_conversation_resolution.enabled,
    lock_branch: .lock_branch.enabled,
    allow_fork_syncing: .allow_fork_syncing.enabled
  }')
  repo=$(gh api "repos/$REPO" --jq '{delete_branch_on_merge}')
  jq -S -n --argjson p "$protection" --argjson r "$repo" '$p + $r'
}

desired() {
  jq -S -n --argjson p "$PROTECTION" --argjson r "$REPO_SETTINGS" '$p + $r'
}

check() {
  if diff <(desired) <(live) >/dev/null; then
    echo "main protection and repo settings match."
  else
    echo "Differences (< wanted, > live):"
    diff <(desired) <(live) || true
    return 1
  fi
}

case "${1:-check}" in
  check)
    check
    ;;
  apply)
    gh api -X PUT "repos/$REPO/branches/main/protection" --input - <<<"$PROTECTION" >/dev/null
    gh api -X PATCH "repos/$REPO" --input - <<<"$REPO_SETTINGS" >/dev/null
    check
    ;;
  *)
    echo "usage: $0 [check|apply]" >&2
    exit 2
    ;;
esac
