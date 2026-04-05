# 06 Customizations vs Vendor

Updated: {DATE}
Owner: repository
Related: [[00-Index]], [[01-System-Overview]], [[03-Data-Model]], [[05-API-Surface]]
Tags: #memory #vendor #customization

## Vendor Baseline
Describe stock behavior expected from the vendor platform.

## Local Customizations
| Area | Custom Behavior | Why It Exists | Risk on Upgrade |
|---|---|---|---|

## File or Module Diff Pointers
- path/to/custom/override
- path/to/patched/module

## Upgrade Risks
- overwritten files
- schema divergence
- hidden coupling

## Notes
Every customization must say why it exists and what breaks if it is removed.
