#!/usr/bin/env bash
# Import contacts from LinkedIn CSV export
# Usage: ./scripts/import-linkedin.sh <Connections.csv>
#
# To get your LinkedIn export:
# 1. Go to linkedin.com/mypreferences/d/download-my-data
# 2. Request archive, wait for email
# 3. Download and extract Connections.csv

set -euo pipefail

CRM_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CSV_FILE="${1:-}"

if [ -z "$CSV_FILE" ] || [ ! -f "$CSV_FILE" ]; then
    echo "Usage: import-linkedin.sh <Connections.csv>"
    exit 1
fi

COUNT=0

# Skip header line, parse CSV
tail -n +2 "$CSV_FILE" | while IFS=',' read -r first_name last_name email company position connected_on _rest; do
    # Clean up fields (remove quotes)
    first_name=$(echo "$first_name" | tr -d '"' | xargs)
    last_name=$(echo "$last_name" | tr -d '"' | xargs)
    email=$(echo "$email" | tr -d '"' | xargs)
    company=$(echo "$company" | tr -d '"' | xargs)
    position=$(echo "$position" | tr -d '"' | xargs)
    connected_on=$(echo "$connected_on" | tr -d '"' | xargs)

    [ -z "$first_name" ] && continue

    FULL_NAME="$first_name $last_name"
    SLUG=$(echo "$FULL_NAME" | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | tr -cd '[:alnum:]-')
    FILE="$CRM_ROOT/contacts/$SLUG.md"

    if [ -f "$FILE" ]; then
        echo "SKIP (exists): $FULL_NAME"
        continue
    fi

    UUID=$(uuidgen | tr '[:upper:]' '[:lower:]')

    cat > "$FILE" << CONTACT
---
id: "$UUID"
name: "$FULL_NAME"
aliases: []
pronouns: ""

# Contact
email:$([ -n "$email" ] && echo "
  - \"$email\"" || echo " []")
phone: []
address: []

# Professional
company: "$company"
role: "$position"
industry: ""
linkedin: ""

# Social
twitter: ""
facebook: ""
instagram: ""
github: ""
website: ""

# Personal
birthday:
interests: []
family: []

# Relationship
how_we_met: "LinkedIn"
met_date:$([ -n "$connected_on" ] && echo " $connected_on" || echo "")
introduced_by: ""
relationship: acquaintance
tags:
  - "linkedin-import"

# CRM
status: active
follow_up_cadence: ""
last_contacted:
next_follow_up:
priority: medium

# Source
source: linkedin
source_id: ""
---

## Notes


## Interaction Log

CONTACT

    echo "ADDED: $FULL_NAME"
    COUNT=$((COUNT + 1))
done

echo "---"
echo "Import complete."
