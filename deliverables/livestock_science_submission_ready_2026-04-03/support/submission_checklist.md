# Submission checklist — ready package

## Authoritative package root
- `deliverables/livestock_science_submission_ready_2026-04-03/`

## Present and authoritative

### Manuscript
- [x] `manuscript/felex_livestock_science_cas.tex` (primary submission — single-column CAS)
- [x] `manuscript/felex_livestock_science_twocol.tex` (double-column CAS preview — cas-dc)
- [x] `manuscript/felex_livestock_science_word.docx` (Word version, generated from authoritative LaTeX)
- [x] `manuscript/felex_references.bib`
- [x] `manuscript/cas-sc.cls`
- [x] `manuscript/cas-dc.cls`
- [x] `manuscript/cas-common.sty`
- [x] `manuscript/cas-model2-names.bst`

### Figures
- [x] `manuscript/Figure_1_System_Architecture.png`
- [x] `manuscript/Figure_2_Quality_Points.png`
- [x] `manuscript/Figure_3_Quality_Delta.png`
- [x] `manuscript/Figure_4_Runtime.png`
- [x] `manuscript/Figure_5_Heatmap.png`
- [x] `manuscript/Figure_6_Cost_Scatter.png`

### Supplementary
- [x] `supplementary/felex_supplementary.tex`

### Russian internal review files
- [x] `internal_review_ru/felex_livestock_science_main_ru.tex`
- [x] `internal_review_ru/felex_supplementary_ru.tex`
- [x] `internal_review_ru/figures/system_architecture_ru.png`

### Benchmark evidence
- [x] `evidence/optimization/benchmark_results.json`
- [x] `evidence/optimization/data_csv/`
- [x] `evidence/agent/qwen4b_4096/agent_benchmark_results.json`
- [x] `evidence/agent/qwen9b_2048/agent_benchmark_results.json`

### Support documents
- [x] `support/Felex_Highlights.docx`
- [x] `support/Felex_Cover_Letter.docx`
- [x] `support/Felex_Declaration_of_Competing_Interests.docx`
- [x] `support/highlights.txt`
- [x] `support/cover_letter.md`

## Intentionally absent / not authoritative
- [x] `Felex_Main_Manuscript.docx` — superseded by `manuscript/felex_livestock_science_word.docx`
- [ ] `Felex_Supplementary_Material.docx` — removed because it was placeholder content
- [ ] `Graphical_Abstract.png` — not present in audited repo

## Verification status

### Phase 1-8 (prior audit)
- [x] Canonical optimization benchmark values reconciled against `.claude/benchmarks/results/benchmark_results.json`
- [x] Agent benchmark values reconciled against full 2026-04-03 4B/4096 and 9B/2048 artifacts
- [x] CAS manuscript citation keys checked against bibliography
- [x] CAS manuscript figure paths checked against packaged assets
- [x] Supplementary workflow summary updated to frozen benchmark values
- [x] Code-side benchmark instrumentation tests/checks passed
- [x] Runtime values reconciled with workflow_summary.csv (Phase 1)
- [x] Feed matrix Table S8 reconciled with ration_matrix.rs (Phase 2)
- [x] LLM table column format fixed — LLL→LLLLLL (Phase 3a)
- [x] Duplicate Ollama sentence removed from Section 2.1 (Phase 3b)
- [x] Abstract trimmed to ≤250 words with 6 structured headings (Phase 4)
- [x] All numerical claims cross-checked against evidence CSVs (Phase 5)
- [x] Supplementary Table S2 spot-checked against case_summary.csv (Phase 6)
- [x] 7 uncited bibliography entries removed (Phase 7)
- [x] Russian internal review files synchronized (Phase 8)
- [x] Corresponding author changed to O.V. Demkina (demkina1976@gmail.com) in all manuscripts
- [x] Russian manuscript reformatted to cas-sc layout matching EN manuscript exactly
- [x] Missing "Заключение" (Conclusions) paragraph added to RU abstract (6/6 headings)
- [x] EN Word manuscript generated from authoritative LaTeX (felex_livestock_science_word.docx)

### Phase 9 (template-based reformatting, 2026-04-05)
- [x] Single-column CAS manuscript reformatted to match Elsevier CAS SC template exactly
- [x] Added `longnamesfirst` to natbib options per CAS template convention
- [x] Added `algorithm`, `algpseudocode` packages (required for Algorithm 1 compilation)
- [x] Added `lineno` package with `\linenumbers` for review submission line numbering
- [x] Removed orphan `\fnmark[1]` from first author (no corresponding `\fntext`)
- [x] Simplified `\cortext[cor1]` to "Corresponding author" (email already via `\ead`)
- [x] Removed duplicate `\affiliation[1]` block (second author references shared affiliation)
- [x] Highlights improved: removed abbreviations ("DSS"), code identifiers, per Elsevier 85-char rules
- [x] Table 1 updated to CAS table format (`[width=\linewidth,cols=6,pos=htbp]`)
- [x] Two-column manuscript completely rewritten using `cas-dc` class (was: article+twocolumn)
- [x] `cas-dc.cls` copied from template to manuscript directory
- [x] DC figures sized: `figure*` for architecture/quality-points/heatmap, `figure` for others
- [x] DC algorithm wrapped in `figure*` for proper two-column spanning
- [x] DC table uses `table*` with `\textwidth` for full-width LLM evaluation table
- [x] DC equations 3-5 split for column-width compatibility
- [x] ORCID corrected: D.D. Kotelnikov → `0009-0003-5159-5796` (all 3 manuscripts)
- [x] ORCID added: O.V. Demkina → `0000-0001-9303-4100` (all 3 manuscripts)
- [x] Orphan `\fnmark[1]` also removed from Russian internal review manuscript
- [x] Supplementary: removed duplicate `\usepackage{booktabs}`, expanded title, updated date
- [x] `highlights.txt` synchronized with manuscript highlights
- [x] Cover letter date updated to April 5, 2026
- [ ] Final LaTeX compile not executed in this environment (no TeX toolchain available)

## Livestock Science (Elsevier) compliance checklist
- [x] Document class: `cas-sc` / `cas-dc` (Elsevier CAS bundle)
- [x] Citation style: author-year (Harvard) via `natbib` + `cas-model2-names.bst`
- [x] Structured abstract with ≤250 words
- [x] Highlights: 5 bullets, each ≤85 characters, no abbreviations, no citations
- [x] Keywords: 6 terms (max 7 allowed)
- [x] Section order: Introduction → Materials and Methods → Results → Discussion → Conclusion
- [x] Declaration of Competing Interest (required)
- [x] Funding statement (required)
- [x] Data Availability Statement (required)
- [x] CRediT author statement via `\credit` + `\printcredits`
- [x] AI use declaration (new Elsevier requirement since 2024)
- [x] Line numbering for review submission
- [x] Both authors have ORCID identifiers
- [x] Corresponding author clearly marked
- [x] Figures embedded as PNG (acceptable: TIFF, EPS, PDF, PNG, JPEG)
- [x] References managed via BibTeX with DOIs where available

## Next external step
- Compile the LaTeX sources on a machine with a TeX toolchain before submission.
- Regenerate Word manuscript from updated LaTeX if portal requires DOCX upload.
- Render graphical abstract at 1328×531 px if required by journal.
