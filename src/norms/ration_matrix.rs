//! Ration matrix engine for feed type proportion constraints.

use super::AnimalNorm;

#[derive(Debug, Clone, PartialEq)]
pub struct RationConstraint {
    pub feed_type: String,
    pub min_pct: f64,
    pub opt_pct: f64,
    pub max_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RationMatrix {
    pub animal_type: String,
    pub constraints: Vec<RationConstraint>,
}

impl RationMatrix {
    pub fn dairy_cattle() -> Self {
        Self {
            animal_type: "cattle_dairy".to_string(),
            constraints: vec![
                ration_constraint("roughage", 35.0, 45.0, 65.0),
                ration_constraint("concentrate", 30.0, 40.0, 60.0),
                ration_constraint("succulent", 5.0, 15.0, 35.0),
                ration_constraint("mineral", 0.5, 1.5, 3.0),
                ration_constraint("npn", 0.0, 0.0, 1.0),
                ration_constraint("animal_origin", 0.0, 0.0, 5.0),
                ration_constraint("premix", 0.0, 1.0, 2.0),
            ],
        }
    }

    pub fn beef_cattle() -> Self {
        Self {
            animal_type: "cattle_beef".to_string(),
            constraints: vec![
                ration_constraint("roughage", 10.0, 25.0, 40.0),
                ration_constraint("concentrate", 55.0, 75.0, 90.0),
                ration_constraint("succulent", 0.0, 10.0, 25.0),
                ration_constraint("mineral", 0.5, 1.5, 2.5),
                ration_constraint("npn", 0.0, 0.0, 1.5),
                ration_constraint("animal_origin", 0.0, 0.0, 0.0),
                ration_constraint("premix", 0.0, 1.0, 1.5),
            ],
        }
    }

    pub fn swine_grower() -> Self {
        Self {
            animal_type: "swine_finisher".to_string(),
            constraints: vec![
                ration_constraint("roughage", 0.0, 0.0, 5.0),
                ration_constraint("concentrate", 90.0, 95.0, 100.0),
                ration_constraint("succulent", 0.0, 0.0, 10.0),
                ration_constraint("mineral", 0.5, 2.0, 3.0),
                ration_constraint("npn", 0.0, 0.0, 0.0),
                ration_constraint("animal_origin", 0.0, 3.0, 8.0),
                ration_constraint("premix", 0.0, 1.0, 2.5),
            ],
        }
    }

    pub fn poultry_broiler() -> Self {
        Self {
            animal_type: "poultry_broiler".to_string(),
            constraints: vec![
                ration_constraint("roughage", 0.0, 0.0, 3.0),
                ration_constraint("concentrate", 95.0, 98.0, 100.0),
                ration_constraint("succulent", 0.0, 0.0, 2.0),
                ration_constraint("mineral", 1.0, 2.0, 5.0),
                ration_constraint("npn", 0.0, 0.0, 0.0),
                ration_constraint("animal_origin", 0.0, 3.0, 10.0),
                ration_constraint("premix", 0.0, 1.0, 3.0),
            ],
        }
    }

    pub fn poultry_layer() -> Self {
        Self {
            animal_type: "poultry_layer".to_string(),
            constraints: vec![
                ration_constraint("roughage", 0.0, 0.0, 5.0),
                ration_constraint("concentrate", 90.0, 93.0, 100.0),
                ration_constraint("succulent", 0.0, 0.0, 5.0),
                ration_constraint("mineral", 2.0, 5.0, 8.0),
                ration_constraint("npn", 0.0, 0.0, 0.0),
                ration_constraint("animal_origin", 0.0, 3.0, 8.0),
                ration_constraint("premix", 0.0, 1.0, 3.0),
            ],
        }
    }

    pub fn for_animal(animal_type: &str) -> Option<Self> {
        match animal_type {
            "cattle_dairy" => Some(Self::dairy_cattle()),
            "cattle_beef" => Some(Self::beef_cattle()),
            "swine" | "swine_finisher" => Some(Self::swine_grower()),
            "poultry" | "poultry_broiler" => Some(Self::poultry_broiler()),
            "poultry_layer" => Some(Self::poultry_layer()),
            _ => None,
        }
    }

    pub fn for_group_id(group_id: &str, species: &str) -> Option<Self> {
        let normalized = group_id.trim().to_ascii_lowercase();

        if normalized.contains("dairy") {
            return Some(Self::dairy_cattle());
        }
        if normalized.contains("beef") {
            return Some(Self::beef_cattle());
        }
        if normalized.contains("layer") {
            return Some(Self::poultry_layer());
        }
        if normalized.contains("broiler") {
            return Some(Self::poultry_broiler());
        }
        if normalized.contains("swine") && !normalized.contains("sow") {
            return Some(Self::swine_grower());
        }

        match species {
            "cattle" => Some(Self::dairy_cattle()),
            "poultry" => Some(Self::poultry_broiler()),
            "swine" => None,
            _ => None,
        }
    }

    pub fn for_norm(norm: &AnimalNorm) -> Option<Self> {
        if matches!(norm.production_type.as_deref(), Some("breeding")) || norm.id.contains("sow") {
            return None;
        }

        Self::for_group_id(norm.id.as_str(), norm.species.as_str())
    }
}

fn ration_constraint(feed_type: &str, min_pct: f64, opt_pct: f64, max_pct: f64) -> RationConstraint {
    RationConstraint {
        feed_type: feed_type.to_string(),
        min_pct,
        opt_pct,
        max_pct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dairy_matrix_uses_source_windows() {
        let matrix = RationMatrix::dairy_cattle();
        let roughage = matrix
            .constraints
            .iter()
            .find(|constraint| constraint.feed_type == "roughage")
            .unwrap();
        let concentrate = matrix
            .constraints
            .iter()
            .find(|constraint| constraint.feed_type == "concentrate")
            .unwrap();

        assert_eq!(roughage.min_pct, 35.0);
        assert_eq!(roughage.max_pct, 65.0);
        assert_eq!(concentrate.min_pct, 30.0);
        assert_eq!(concentrate.max_pct, 60.0);
    }

    #[test]
    fn for_animal_returns_expected_poultry_matrix() {
        let matrix = RationMatrix::for_animal("poultry_layer").unwrap();
        assert_eq!(matrix.animal_type, "poultry_layer");
        assert!(matrix
            .constraints
            .iter()
            .any(|constraint| constraint.feed_type == "mineral" && constraint.max_pct == 8.0));
    }

    #[test]
    fn sow_norms_do_not_reuse_finisher_matrix() {
        let sow = AnimalNorm {
            id: "swine_sow_lactating".to_string(),
            species: "swine".to_string(),
            production_type: Some("breeding".to_string()),
            ..Default::default()
        };

        assert!(RationMatrix::for_norm(&sow).is_none());
    }
}
