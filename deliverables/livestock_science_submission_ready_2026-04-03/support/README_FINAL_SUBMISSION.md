# Felex submission support notes

This support folder accompanies the authoritative ready package at:

- `deliverables/livestock_science_submission_ready_2026-04-03/`

## What is authoritative in the ready package

### Manuscript sources
- `../manuscript/felex_livestock_science_cas.tex`
- `../manuscript/felex_references.bib`
- `../manuscript/cas-sc.cls`
- `../manuscript/cas-common.sty`
- `../manuscript/cas-model2-names.bst`

### Supplementary sources
- `../supplementary/felex_supplementary.tex`

### Internal Russian review sources
- `../internal_review_ru/felex_livestock_science_main_ru.tex`
- `../internal_review_ru/felex_supplementary_ru.tex`

### Benchmark evidence
- `../evidence/optimization/benchmark_results.json`
- `../evidence/optimization/data_csv/`
- `../evidence/agent/qwen4b_4096/agent_benchmark_results.json`
- `../evidence/agent/qwen9b_2048/agent_benchmark_results.json`

## Support documents present
- `Felex_Highlights.docx`
- `Felex_Cover_Letter.docx`
- `Felex_Declaration_of_Competing_Interests.docx`
- `highlights.txt`
- `cover_letter.md`
- `submission_checklist.md`

## Important omissions

- `Felex_Main_Manuscript.docx` is not present in the audited repository.
- `Felex_Supplementary_Material.docx` was removed from the ready package because it was a placeholder document, not a finalized supplementary file.
- `Graphical_Abstract.png` is not present in the audited repository.

## Frozen benchmark values used in the ready package

### Optimization benchmark
- 23 executed scenarios
- 69 workflow runs
- Mean runtime: `246.6 ms`
- Mean hard-pass rate: `64.824%`
- Mean norm coverage index: `81.780`

### Agent benchmark
- Qwen 3.5 4B @ 4096: `34.17/100`
- Qwen 3.5 9B @ 2048: `34.17/100`
- No hidden model/context fallback observed in either full artifact

## Current limits

- This environment did not have `pdflatex`, `xelatex`, `latexmk`, `bibtex`, or `texlab`, so the LaTeX sources were repaired and statically checked, but not fully compiled here.
- If the journal workflow requires Word-format manuscript upload, the English main manuscript DOCX still needs to be generated from the authoritative source.
- A Word manuscript (`Felex_Main_Manuscript.docx`) should be generated from the authoritative LaTeX source for journal portals requiring DOCX upload.
- A graphical abstract concept has been prepared and needs to be rendered at 1328×531 px in a vector editor before submission.
