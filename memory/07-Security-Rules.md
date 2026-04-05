# 07 Security Rules

Updated: {DATE}
Owner: repository
Related: [[00-Index]], [[05-API-Surface]], [[03-Data-Model]]
Tags: #memory #security #risk

## Authentication Rules
Document how users and systems prove identity.

## Authorization Rules
Document what each actor is allowed to do.

## Sensitive Data Handling
Note storage, masking, transmission, and retention rules.

## Logging Constraints
State what must never be logged.

## Secret Boundaries
State where secrets live and where they must not be copied.

## Unsafe Change Patterns
- direct bypass of auth checks
- logging sensitive payloads
- trusting staged content without validation
