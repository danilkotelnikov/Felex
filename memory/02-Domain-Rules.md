# 02 Domain Rules

**Updated:** 2026-03-28
**Owner:** repository
**Related:** [[00-Index]], [[03-Data-Model]], [[08-User-Workflows]], [[11-Glossary]]
**Tags:** #memory #domain #rules

## Purpose

Capture business rules and non-obvious implementation rules for the Felex feed ration formulation system.

---

## Core Business Rules

### Rule 1: Nutrient Constraint Hierarchy

**Priority:** Critical
**Enforced by:** `src/diet_engine/optimizer.rs`

| Tier | Purpose | Examples | Relaxation Order |
|------|---------|----------|------------------|
| Hard | Safety, toxicity | Selenium max, mycotoxins | Never relaxed |
| Tier 1 | Core nutrition | Energy, protein, Ca, P | Relaxed last |
| Tier 2 | Performance | Amino acids, fiber | Relaxed before Tier 1 |
| Tier 3 | Optimization | Vitamins, trace minerals | Relaxed first |

**Inputs:** Animal norms, feed composition, user constraints
**Conditions:** LP solver detects infeasibility
**Exceptions:** User-locked feeds cannot be removed
**Consequences:** Lower-tier constraints relaxed iteratively until feasible solution found
**Evidence:** `src/diet_engine/optimizer.rs` constraint_tier_for_key() function

---

### Rule 2: Species Feed Filtering

**Priority:** Critical
**Enforced by:** `src/diet_engine/feed_groups.rs`

```
IF animal_species = "cattle"
THEN filter feeds WHERE is_feed_allowed_for_cattle = true
```

**Inputs:** Animal group species, feed eligibility flags
**Conditions:** Feed must be eligible for target species
**Exceptions:** Custom feeds may override eligibility
**Consequences:** Ineligible feeds excluded from optimization
**Evidence:** `is_feed_allowed_for_context()` function

---

### Rule 3: Dry Matter Basis Calculations

**Priority:** High
**Enforced by:** `src/diet_engine/nutrient_calc.rs`

All nutrient percentages are calculated on dry matter basis:

```
nutrient_pct_dm = (nutrient_amount / total_dm_kg) * 100
```

**Inputs:** Feed amounts, dry matter percentages, nutrient values
**Conditions:** Total DM > 0
**Exceptions:** Division by zero returns 0
**Consequences:** All nutrient status displays use % DM
**Evidence:** `NutrientSummary` struct calculations

---

### Rule 4: Price Fallback Chain

**Priority:** High
**Enforced by:** `src/diet_engine/economics.rs`

```
1. Use direct feed_prices table value
2. If missing, use category average price
3. If missing, mark as unpriced (cost = 0)
```

**Inputs:** Feed ID, price database
**Conditions:** Price lookup during cost calculation
**Exceptions:** Unpriced feeds included but cost = 0
**Consequences:** Economic optimization may favor unpriced feeds
**Evidence:** `calculate_economics()` function

---

### Rule 5: Norm Resolution Order

**Priority:** High
**Enforced by:** `src/norms/mod.rs`

```
1. Check dynamic factorial norms (weight, milk, gain)
2. Check breed-adjusted norms
3. Fall back to base category norms
4. Return error if no match
```

**Inputs:** Animal properties (species, stage, weight, milk, gain)
**Conditions:** Norm lookup for optimization target
**Exceptions:** Unknown species/stage combinations
**Consequences:** Optimization fails without valid norms
**Evidence:** `NormResolver::resolve()` function

---

## Derived Rules

### Rule D1: Energy EKE is Derived

**Status:** Implemented
**Source:** `src/norms/cattle_beef.rs`

EKE (Кормовые Единицы) is NOT stored in database. It is calculated from OE:

```
energy_eke = energy_oe_cattle / 10.47
```

**Implication:** Frontend displays EKE but backend uses OE

---

### Rule D2: Ca:P Ratio is Calculated

**Status:** Implemented
**Source:** `src/diet_engine/nutrient_calc.rs`

Ca:P ratio is NOT a stored nutrient. Calculated as:

```
ca_p_ratio = calcium / phosphorus  (when phosphorus > 0)
```

**Implication:** Ratio validation occurs post-calculation

---

### Rule D3: Carotene Does Not Convert to Vitamin A

**Status:** Enforced (2026-03-28)
**Source:** `src/nutrients/conversions.rs`

Carotene-to-vitamin A conversion is NOT implemented because:
- Conversion factor varies by animal type (400 IU/mg cattle, 300 swine, 250 poultry)
- Conversion depends on diet composition
- Conversion efficiency varies by physiological state

**Implication:** Vitamin A must be measured directly; removed from system

---

### Rule D4: Fiber Scope Is Explicitly Limited

**Status:** Known limitation
**Source:** `src/nutrients/manifest.rs`

The system currently uses only the implemented fiber key set present in the active nutrient manifest and optimizer routing.

**Implication:** Fiber analysis scope is bounded by the implemented key set in current release.

---

## Enforcement Notes

| Rule | Workflow | Service | Data Model |
|------|----------|---------|------------|
| Constraint Hierarchy | Optimize | `diet_engine/optimizer.rs` | `AnimalNorm` |
| Species Filtering | All | `diet_engine/feed_groups.rs` | `feeds` table |
| DM Calculations | All | `diet_engine/nutrient_calc.rs` | `NutrientSummary` |
| Price Fallback | Economics | `diet_engine/economics.rs` | `feed_prices` |
| Norm Resolution | All | `norms/mod.rs` | `animal_norms` |

---

## Known Limitations (Not Yet Implemented)

| Limitation | Impact | Future Work |
|------------|--------|-------------|
| Limited fiber-key scope | Reduced detail in fiber assessment | Future schema+optimizer expansion |
| No SID/TID amino acids | Swine/poultry less optimized | Backend implementation |
| No rumen simulation | Dairy less precise than CNCPS | Major enhancement |
| No environmental constraints | N/P excretion not tracked | Regulatory requirement |
| No multi-objective optimization | Single cost objective only | Pareto optimization |

---

## Open Questions

1. **Dynamic norm updates:** Should norms update automatically when new research published?
2. **Regional price normalization:** Should prices be normalized for regional differences?
3. **Feed substitution rules:** Should there be hard limits on feed substitution rates?
