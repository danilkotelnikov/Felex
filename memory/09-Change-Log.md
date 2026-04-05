# 09 Change Log

Updated: 2026-04-04
Owner: repository
Related: [[00-Index]], [[10-Decision-Records]], [[14-Session-Inbox]]
Tags: #memory #change-log #history

## 2026-04-04 — Manuscript–Code Reconciliation (10-Phase)

### Summary
Full reconciliation of the Livestock Science submission manuscript against authoritative benchmark evidence CSVs and source code. All numerical claims in the manuscript, supplementary, highlights, and Russian internal review files now trace to `workflow_summary.csv` and `ration_matrix.rs`.

### Changed
- **Runtime values** (Phase 1): Updated across all files from stale benchmark run (213.8/534.4/106.6/0.36 ms) to authoritative CSV values (246.6/617.5/121.8/0.41 ms)
- **Feed matrix Table S8** (Phase 2): Corrected 4 mismatched constraint values in EN and RU supplementary to match `src/norms/ration_matrix.rs` (dairy succulent opt 20→15, swine mineral 0/1/4→0.5/2/3, poultry concentrate 90/96→95/98, poultry mineral 0/1/4→1/2/5)
- **LLM evaluation table** (Phase 3a): Fixed column format from 3 to 6 columns (`LLL` → `LLLLLL`)
- **Duplicate sentence** (Phase 3b): Removed duplicated Ollama agent sentence from Section 2.1
- **Abstract** (Phase 4): Trimmed from ~333 to ~229 words, added missing Conclusions heading (now 6/6 required headings)
- **Bibliography** (Phase 7): Removed 7 uncited entries (amts2023, cpm2022, nds2023, kormmix2020, oliveira2024, tedeschi2005, nsabiyeze2025)
- **Russian files** (Phase 8): All runtime and matrix values synchronized
- **README_FINAL_SUBMISSION.md** (Phase 9): Updated frozen runtime value, added graphical abstract and DOCX generation notes

### Verified
- Coverage values (83.2, 85.0, 77.1, grand mean 81.8) match `workflow_summary.csv`
- Cost grand mean (183.0 RUB) matches CSV
- Hard-constraint pass rate (64.8%) matches Table S3 average
- Agent benchmark metrics (recall 0.17, grounded 0.67, lookup 0.50, applicability 34.2) match both JSON artifacts
- Table S2 per-scenario values spot-checked against `case_summary.csv` — all correct
- Dairy roughage claim (35–65%) matches `dairy_cattle()` code
- No stale values remaining (grep-verified)

### Affected Files
- `manuscript/felex_livestock_science_cas.tex`
- `manuscript/felex_references.bib`
- `supplementary/felex_supplementary.tex`
- `internal_review_ru/felex_livestock_science_main_ru.tex`
- `internal_review_ru/felex_supplementary_ru.tex`
- `support/highlights.txt` (no changes needed — no runtime values present)
- `support/README_FINAL_SUBMISSION.md`
- `support/submission_checklist.md`

---

## 2026-03-28 — Phase 2 Cleanup (Vitamin A Removal)

### Added
- Selenium retained in canonical manifest (direct database values)
- SID/TID amino acids marked as PLANNED in frontend
- Manuscript validation report (`.claude/audit/Manuscript_Validation_Report.md`)
- Memory notes with Mermaid diagrams (`[[00-Index]]`, `[[01-System-Overview]]`, `[[03-Data-Model]]`)
- Domain rules documentation (`[[02-Domain-Rules]]`)
- API surface documentation (`[[05-API-Surface]]`)

### Changed
- Nutrient count: 31 → 30 canonical nutrients (vitamin_a removed)
- `memory/00-Index.md` — Comprehensive navigation with cleanup status
- `memory/01-System-Overview.md` — Architecture, limitations, performance metrics
- `memory/03-Data-Model.md` — ER diagram, schema, dangerous assumptions
- `memory/02-Domain-Rules.md` — Business rules, derived rules, limitations
- `memory/05-API-Surface.md` — Full API endpoint documentation

### Removed
- **Vitamin A** from `src/nutrients/manifest.rs` (factorial-dependent conversion unreliable)
- **Vitamin A** from `frontend/src/types/nutrient.ts` (NUTRIENT_DEFS, NORMS_BY_GROUP)
- **Vitamin A** from `frontend/src/lib/nutrient-registry.ts` (BASE_NUTRIENTS, MANAGED_KEYS)
- **unsupported fiber-fraction references** from `src/agent/prompt.rs` (hallucination risk)
- **unsupported fiber-fraction references** from `src/nutrients/categories.rs` comment
- **LowNDF warning variant** from `src/diet_engine/mod.rs` (dead code)
- **Carotene→Vitamin A conversion** from `src/nutrients/conversions.rs` (reverted to empty)
- **PLANNED_BY_KEY export** from frontend (simplified)

### Affected Notes
- `[[03-Data-Model]]` — Nutrient schema updated (31 → 30 nutrients)
- `[[01-System-Overview]]` — Known limitations updated
- `[[09-Change-Log]]` — This entry
- `[[02-Domain-Rules]]` — New rule D3: No carotene conversion

### Risks
- **Manuscript discrepancies:** Claims (847ms, 89.1 coverage) don't match benchmark data
- **i18n files:** May contain unsupported fiber-fraction references requiring removal
- **Benchmark hard_pass scale:** Values >100 need verification

### Rationale for Vitamin A Removal

Vitamin A was removed because:
1. **Factorial-dependent conversion:** Carotene→Vit A varies by animal type, diet, physiological state
2. **No reliable conversion factor:** 400 IU/mg (cattle), 300 (swine), 250 (poultry) too approximate
3. **Direct measurement required:** Feed samples must be analyzed for Vit A content
4. **Schema inconsistency:** Database column exists but values not reliably populated

### Follow-up
- [ ] Update manuscript claims to match benchmark data
- [ ] Verify i18n files for unsupported fiber-fraction references
- [ ] Complete remaining memory notes (04, 06-08, 10-15)
- [ ] Create final technical documentation (RU/EN)
- [ ] Cross-validate all benchmark statistics

---

## 2026-03-26 — Phase 1 Remediation

## 2026-03-26 — Initial Scaffold

### Changed
- initial scaffold installed

### Affected Notes
- [[00-Index]]
- [[13-Operating-Rules]]

### Risks
- memory notes still need project-specific facts

### Follow-up
- populate the canonical notes with repository facts
