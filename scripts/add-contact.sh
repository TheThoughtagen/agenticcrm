#!/usr/bin/env bash
# Add a new contact from the template
# Usage: ./scripts/add-contact.sh "First Last"

set -euo pipefail

CRM_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TEMPLATE="$CRM_ROOT/templates/contact.md"

if [ $# -lt 1 ]; then
    echo "Usage: add-contact.sh \"First Last\""
    exit 1
fi

NAME="$1"
SLUG=$(echo "$NAME" | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | tr -cd '[:alnum:]-')
FILE="$CRM_ROOT/contacts/$SLUG.md"

if [ -f "$FILE" ]; then
    echo "Contact already exists: $FILE"
    exit 1
fi

UUID=$(uuidgen | tr '[:upper:]' '[:lower:]')

sed "s/{{uuid}}/$UUID/" "$TEMPLATE" | sed "s/^name: \"\"/name: \"$NAME\"/" > "$FILE"

echo "Created: $FILE"
echo "Edit with: \$EDITOR $FILE"
