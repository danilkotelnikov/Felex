# 13 Operating Rules

Updated: {DATE}
Owner: repository
Related: [[00-Index]], [[14-Session-Inbox]], [[09-Change-Log]]
Tags: #memory #rules #operations

## Global Formatting Rules
- Use Obsidian wiki links in the form `[[Note Name]]` or `[[Note Name#Section]]`.
- Keep the metadata header with `Updated`, `Owner`, `Related`, and `Tags`.
- Use stable heading levels. Do not skip heading depth.
- Use Mermaid for diagrams.
- Use tables only for entities, APIs, comparisons, or risk matrices.
- Every canonical note should link to at least two other notes where applicable.
- Facts must be concrete, testable, and traceable to evidence.

## Global Operating Rules
- Read relevant notes before making cross-layer changes.
- Keep canonical memory separate from session observations.
- Surface contradictions explicitly.
- Update memory when architecture, workflows, schema, security, or vendor differences change.
- Do not rename notes casually. Preserve link stability.

## 01 System Overview
### Formatting Rules
- keep it high level
- include a Mermaid component map
- link outward to deeper notes
### Operating Rules
- update only when the architecture materially changes
- do not dump endpoint or field-level detail here

## 02 Domain Rules
### Formatting Rules
- one rule per subsection
- list inputs, conditions, exceptions, consequences, and evidence
### Operating Rules
- separate verified rules from inferred rules
- link each rule to impacted workflows or code

## 03 Data Model
### Formatting Rules
- maintain entity table and relationship list
- keep storage peculiarities and lifecycle states explicit
### Operating Rules
- record only verified schema behavior
- update after migrations, content storage changes, or versioning changes

## 04 Content Lifecycle
### Formatting Rules
- express lifecycle as ordered steps
- include transformation rules and failure modes
### Operating Rules
- link each step to the code, job, or process that performs it
- keep lifecycle detail separate from raw schema detail

## 05 API Surface
### Formatting Rules
- group by user-facing capability
- list route, controller, service, dependency, and auth
### Operating Rules
- mark routing anomalies explicitly
- use this note to bridge workflows and backend implementation

## 06 Customizations vs Vendor
### Formatting Rules
- keep baseline and local behavior clearly separated
- use a comparison table for local customizations
### Operating Rules
- state why each customization exists
- state upgrade risk for each customization
- consult before vendor-adjacent edits

## 07 Security Rules
### Formatting Rules
- separate authentication, authorization, data handling, logging, and secrets
- include an unsafe change patterns section
### Operating Rules
- read before changing endpoints, auth, logging, or sensitive data flows
- prefer explicit prohibitions over vague guidance

## 08 User Workflows
### Formatting Rules
- use one subsection per workflow
- map each user step to technical touchpoints
- include a diagram for major flows
### Operating Rules
- reason from user action to API to data layer
- update when business flow or control flow changes

## 09 Change Log
### Formatting Rules
- log meaningful architectural or behavioral changes only
- list affected notes and follow-up work
### Operating Rules
- do not mirror every Git commit
- keep entries concise and concrete

## 10 Decision Records
### Formatting Rules
- one decision per record
- include context, decision, consequences, and related notes
### Operating Rules
- use to prevent repeated re-litigation of settled choices
- update status if a decision is superseded

## 11 Glossary
### Formatting Rules
- define terms tersely
- record naming conflicts explicitly
### Operating Rules
- update when terminology changes or ambiguity appears
- prefer project-native terms over generic synonyms

## 12 Dependency Map
### Formatting Rules
- use arrow notation consistently
- record code dependencies, knowledge dependencies, and risk chains
### Operating Rules
- update when major module relationships or risk chains change
- consult during impact analysis

## 14 Session Inbox
### Formatting Rules
- each session observation needs status, target notes, evidence, observation, and action
- keep entries dated
### Operating Rules
- treat all entries as tentative until promoted
- do not cite this note as canonical truth
- reject or promote entries promptly to prevent drift
