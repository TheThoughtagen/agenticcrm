#!/usr/bin/env bash
# List contacts due for follow-up
# Usage: ./scripts/due-followups.sh [YYYY-MM-DD]

set -euo pipefail

CRM_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TODAY="${1:-$(date +%Y-%m-%d)}"

echo "Follow-ups due by $TODAY:"
echo "---"

for file in "$CRM_ROOT"/contacts/*.md; do
    [ -f "$file" ] || continue
    [[ "$(basename "$file")" == ".gitkeep" ]] && continue

    next=$(grep '^next_follow_up:' "$file" | head -1 | sed 's/next_follow_up: *//')
    [ -z "$next" ] && continue

    if [[ "$next" < "$TODAY" || "$next" == "$TODAY" ]]; then
        name=$(grep '^name:' "$file" | head -1 | sed 's/name: *"\(.*\)"/\1/')
        last=$(grep '^last_contacted:' "$file" | head -1 | sed 's/last_contacted: *//')
        echo "$next | $name | last: $last | $file"
    fi
done | sort
