#!/usr/bin/env bash
# Search contacts by name, tag, company, or any field
# Usage: ./scripts/search.sh <query>

set -euo pipefail

CRM_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if [ $# -lt 1 ]; then
    echo "Usage: search.sh <query>"
    exit 1
fi

QUERY="$1"

grep -rl --include="*.md" -i "$QUERY" "$CRM_ROOT/contacts/" 2>/dev/null | while read -r file; do
    # Extract name from frontmatter
    name=$(grep '^name:' "$file" | head -1 | sed 's/name: *"\(.*\)"/\1/')
    company=$(grep '^company:' "$file" | head -1 | sed 's/company: *"\(.*\)"/\1/')
    status=$(grep '^status:' "$file" | head -1 | sed 's/status: *//')
    echo "$name | $company | $status | $file"
done
