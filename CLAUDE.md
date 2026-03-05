# AgenticCRM - Agent Instructions

This is a plain-text personal CRM. Contacts are markdown files with YAML frontmatter in `contacts/`.

## Working with contacts

- Schema definition: `.schemas/contact.yaml`
- Contact template: `templates/contact.md`
- Each contact file: `contacts/firstname-lastname.md`
- File naming: lowercase, hyphen-separated, alphanumeric only

## Adding a contact

1. Copy template, replace `{{uuid}}` with a new UUID, fill in fields
2. Or run: `./scripts/add-contact.sh "First Last"`

## Logging an interaction

Append to the contact's `## Interaction Log` section in reverse chronological order:

```markdown
### YYYY-MM-DD | type | short summary

Free-form notes about the interaction.
```

Interaction types: coffee, call, email, message, conference, meeting, lunch, intro

## Updating CRM fields

After logging an interaction, update frontmatter:
- `last_contacted` to today's date
- `next_follow_up` based on `follow_up_cadence`
- `status` if relationship state changed

## Import sources

- LinkedIn: `./scripts/import-linkedin.sh <Connections.csv>`
- Other imports land in `imports/` for processing

## Conventions

- Dates are always YYYY-MM-DD
- Tags are lowercase, hyphenated
- Empty fields use `""` for strings, `[]` for arrays, leave blank for dates
- Keep frontmatter fields in the order defined in the template
