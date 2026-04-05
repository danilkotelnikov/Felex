use super::{PresetAnimalParams, PresetCategory, PresetSubcategory};

fn subcategory(
    id: &str,
    name_ru: &str,
    name_en: &str,
    animal_group_id: &str,
    norm_preset_id: Option<&str>,
    legacy_preset_id: Option<&str>,
    params: PresetAnimalParams,
    research_source: Option<&str>,
    feed_recommendations: &[&str],
) -> PresetSubcategory {
    PresetSubcategory {
        id: id.to_string(),
        name_ru: name_ru.to_string(),
        name_en: name_en.to_string(),
        animal_group_id: animal_group_id.to_string(),
        norm_preset_id: norm_preset_id.map(ToOwned::to_owned),
        legacy_preset_id: legacy_preset_id.map(ToOwned::to_owned),
        params,
        research_source: research_source.map(ToOwned::to_owned),
        feed_recommendations: feed_recommendations.iter().map(|key| key.to_string()).collect(),
    }
}

pub fn preset_categories() -> Vec<PresetCategory> {
    vec![
        PresetCategory {
            species: "cattle".to_string(),
            production_type: "dairy".to_string(),
            subcategories: vec![
                subcategory(
                    "dairy_high_yield",
                    "Высокопродуктивные (35+ кг/день)",
                    "High-Yield (35+ kg/day)",
                    "cattle_dairy",
                    Some("cattle_dairy_35"),
                    Some("dairy_35"),
                    PresetAnimalParams {
                        milk_yield_kg: Some(40.0),
                        live_weight_kg: Some(650.0),
                        lactation_stage: Some("early".to_string()),
                        ..Default::default()
                    },
                    Some("NASEM Dairy 2021, Table 7-1"),
                    &["corn_silage", "alfalfa_hay", "soybean_meal", "corn_grain"],
                ),
                subcategory(
                    "dairy_moderate",
                    "Средняя продуктивность (20-25 кг/день)",
                    "Moderate (20-25 kg/day)",
                    "cattle_dairy",
                    Some("cattle_dairy_25"),
                    Some("dairy_25"),
                    PresetAnimalParams {
                        milk_yield_kg: Some(22.0),
                        live_weight_kg: Some(600.0),
                        lactation_stage: Some("mid".to_string()),
                        ..Default::default()
                    },
                    Some("Russian Dairy Norms 2018"),
                    &["grass_silage", "hay", "barley", "sunflower_meal"],
                ),
                subcategory(
                    "dairy_dry",
                    "Сухостойные",
                    "Dry Cows",
                    "cattle_dairy",
                    None,
                    None,
                    PresetAnimalParams {
                        milk_yield_kg: Some(0.0),
                        live_weight_kg: Some(700.0),
                        days_pregnant: Some(250),
                        ..Default::default()
                    },
                    Some("NASEM Dairy 2021, Chapter 10"),
                    &["grass_hay", "straw", "mineral_premix"],
                ),
                subcategory(
                    "dairy_transition",
                    "Транзитный период",
                    "Transition Period",
                    "cattle_dairy",
                    None,
                    None,
                    PresetAnimalParams {
                        milk_yield_kg: Some(0.0),
                        live_weight_kg: Some(680.0),
                        days_to_calving: Some(21),
                        ..Default::default()
                    },
                    Some("Transition Cow Management Guidelines"),
                    &["close_up_ration", "anionic_salts"],
                ),
            ],
        },
        PresetCategory {
            species: "cattle".to_string(),
            production_type: "beef".to_string(),
            subcategories: vec![
                subcategory(
                    "beef_backgrounding",
                    "Доращивание (200-350 кг)",
                    "Backgrounding (200-350 kg)",
                    "cattle_beef",
                    None,
                    None,
                    PresetAnimalParams {
                        live_weight_kg: Some(280.0),
                        daily_gain_g: Some(1200.0),
                        ..Default::default()
                    },
                    Some("NASEM Beef 2016"),
                    &["pasture", "hay", "grain_supplement"],
                ),
                subcategory(
                    "beef_growing",
                    "Откорм рост (350-500 кг)",
                    "Growing (350-500 kg)",
                    "cattle_beef",
                    None,
                    Some("beef_400"),
                    PresetAnimalParams {
                        live_weight_kg: Some(420.0),
                        daily_gain_g: Some(1400.0),
                        ..Default::default()
                    },
                    None,
                    &["corn_silage", "corn", "protein_supplement"],
                ),
                subcategory(
                    "beef_finishing",
                    "Финишный откорм (500-700 кг)",
                    "Finishing (500-700 kg)",
                    "cattle_beef",
                    None,
                    Some("beef_550"),
                    PresetAnimalParams {
                        live_weight_kg: Some(600.0),
                        daily_gain_g: Some(1200.0),
                        ..Default::default()
                    },
                    None,
                    &["high_energy_grain", "corn", "fat_supplement"],
                ),
                subcategory(
                    "beef_heavy",
                    "Тяжёлый откорм (700-1000 кг)",
                    "Heavy Finishing (700-1000 kg)",
                    "cattle_beef",
                    None,
                    None,
                    PresetAnimalParams {
                        live_weight_kg: Some(850.0),
                        daily_gain_g: Some(900.0),
                        ..Default::default()
                    },
                    None,
                    &["maintenance_ration", "controlled_energy"],
                ),
                subcategory(
                    "beef_max_weight",
                    "Максимальный вес (1000-1200 кг)",
                    "Maximum Weight (1000-1200 kg)",
                    "cattle_beef",
                    None,
                    None,
                    PresetAnimalParams {
                        live_weight_kg: Some(1100.0),
                        daily_gain_g: Some(600.0),
                        ..Default::default()
                    },
                    None,
                    &["holding_ration"],
                ),
            ],
        },
        PresetCategory {
            species: "swine".to_string(),
            production_type: "growing".to_string(),
            subcategories: vec![
                subcategory(
                    "swine_prestarter",
                    "Престартер (5-10 кг)",
                    "Prestarter (5-10 kg)",
                    "swine_finisher",
                    None,
                    None,
                    PresetAnimalParams {
                        live_weight_kg: Some(7.0),
                        daily_gain_g: Some(350.0),
                        ..Default::default()
                    },
                    Some("NRC Swine 2012"),
                    &["prestarter_complete", "milk_replacer"],
                ),
                subcategory(
                    "swine_starter",
                    "Стартер (10-25 кг)",
                    "Starter (10-25 kg)",
                    "swine_finisher",
                    Some("swine_starter"),
                    Some("swine_starter"),
                    PresetAnimalParams {
                        live_weight_kg: Some(18.0),
                        daily_gain_g: Some(500.0),
                        ..Default::default()
                    },
                    None,
                    &["starter_complete", "soybean_meal"],
                ),
                subcategory(
                    "swine_grower",
                    "Ростовой (25-60 кг)",
                    "Grower (25-60 kg)",
                    "swine_finisher",
                    None,
                    None,
                    PresetAnimalParams {
                        live_weight_kg: Some(45.0),
                        daily_gain_g: Some(850.0),
                        ..Default::default()
                    },
                    None,
                    &["corn", "wheat", "soybean_meal", "lysine"],
                ),
                subcategory(
                    "swine_finisher_1",
                    "Финишер I (60-90 кг)",
                    "Finisher I (60-90 kg)",
                    "swine_finisher",
                    Some("swine_finisher_preset"),
                    Some("swine_finisher"),
                    PresetAnimalParams {
                        live_weight_kg: Some(75.0),
                        daily_gain_g: Some(950.0),
                        ..Default::default()
                    },
                    None,
                    &["corn", "barley", "sunflower_meal"],
                ),
                subcategory(
                    "swine_finisher_2",
                    "Финишер II (90-120 кг)",
                    "Finisher II (90-120 kg)",
                    "swine_finisher",
                    Some("swine_finisher_preset"),
                    Some("swine_finisher"),
                    PresetAnimalParams {
                        live_weight_kg: Some(105.0),
                        daily_gain_g: Some(850.0),
                        ..Default::default()
                    },
                    None,
                    &["barley", "wheat", "reduced_protein"],
                ),
            ],
        },
        PresetCategory {
            species: "swine".to_string(),
            production_type: "breeding".to_string(),
            subcategories: vec![
                subcategory(
                    "swine_gilt",
                    "Ремонтные свинки",
                    "Replacement Gilts",
                    "swine_sow",
                    None,
                    None,
                    PresetAnimalParams {
                        live_weight_kg: Some(130.0),
                        daily_gain_g: Some(700.0),
                        ..Default::default()
                    },
                    None,
                    &["gilt_developer"],
                ),
                subcategory(
                    "swine_gestation",
                    "Супоросные свиноматки",
                    "Gestating Sows",
                    "swine_sow",
                    Some("swine_sow_gestation"),
                    Some("swine_sow_gestation"),
                    PresetAnimalParams {
                        live_weight_kg: Some(200.0),
                        days_pregnant: Some(80),
                        ..Default::default()
                    },
                    None,
                    &["gestation_ration", "fiber_source"],
                ),
                subcategory(
                    "swine_lactation",
                    "Подсосные свиноматки",
                    "Lactating Sows",
                    "swine_sow",
                    Some("swine_sow_lactation"),
                    Some("swine_sow_lactation"),
                    PresetAnimalParams {
                        live_weight_kg: Some(180.0),
                        piglets: Some(12),
                        ..Default::default()
                    },
                    None,
                    &["high_energy", "high_lysine"],
                ),
            ],
        },
        PresetCategory {
            species: "poultry".to_string(),
            production_type: "broiler".to_string(),
            subcategories: vec![
                subcategory(
                    "broiler_starter",
                    "Стартер (0-10 дней)",
                    "Starter (0-10 days)",
                    "poultry_broiler",
                    Some("poultry_broiler_starter"),
                    Some("broiler_starter"),
                    PresetAnimalParams {
                        age_days: Some(5),
                        target_weight_g: Some(250),
                        ..Default::default()
                    },
                    Some("Ross 308 Manual 2022"),
                    &["broiler_starter_crumble"],
                ),
                subcategory(
                    "broiler_grower",
                    "Ростовой (11-24 дня)",
                    "Grower (11-24 days)",
                    "poultry_broiler",
                    Some("poultry_broiler_grower"),
                    None,
                    PresetAnimalParams {
                        age_days: Some(18),
                        target_weight_g: Some(900),
                        ..Default::default()
                    },
                    None,
                    &["broiler_grower_pellet"],
                ),
                subcategory(
                    "broiler_finisher",
                    "Финишер (25-42 дня)",
                    "Finisher (25-42 days)",
                    "poultry_broiler",
                    Some("poultry_broiler_finisher"),
                    Some("broiler_finisher"),
                    PresetAnimalParams {
                        age_days: Some(35),
                        target_weight_g: Some(2200),
                        ..Default::default()
                    },
                    None,
                    &["broiler_finisher_pellet"],
                ),
            ],
        },
        PresetCategory {
            species: "poultry".to_string(),
            production_type: "layer".to_string(),
            subcategories: vec![
                subcategory(
                    "layer_pullet",
                    "Молодки (0-18 недель)",
                    "Pullets (0-18 weeks)",
                    "poultry_layer",
                    None,
                    None,
                    PresetAnimalParams {
                        age_weeks: Some(12),
                        ..Default::default()
                    },
                    None,
                    &["pullet_developer"],
                ),
                subcategory(
                    "layer_prelay",
                    "Предкладка (18-20 недель)",
                    "Pre-lay (18-20 weeks)",
                    "poultry_layer",
                    None,
                    None,
                    PresetAnimalParams {
                        age_weeks: Some(19),
                        ..Default::default()
                    },
                    None,
                    &["prelay_ration", "calcium_buildup"],
                ),
                subcategory(
                    "layer_peak",
                    "Пик яйценоскости (20-45 недель)",
                    "Peak Production (20-45 weeks)",
                    "poultry_layer",
                    Some("poultry_layer_phase1"),
                    Some("layer_phase1"),
                    PresetAnimalParams {
                        age_weeks: Some(30),
                        production_pct: Some(95.0),
                        ..Default::default()
                    },
                    None,
                    &["layer_peak", "oyster_shell"],
                ),
                subcategory(
                    "layer_late",
                    "Вторая фаза (45-70 недель)",
                    "Phase 2 (45-70 weeks)",
                    "poultry_layer",
                    Some("poultry_layer_phase2"),
                    Some("layer_phase2"),
                    PresetAnimalParams {
                        age_weeks: Some(55),
                        production_pct: Some(85.0),
                        ..Default::default()
                    },
                    None,
                    &["layer_phase2", "reduced_protein"],
                ),
                subcategory(
                    "layer_post_peak",
                    "После пика (70+ недель)",
                    "Post-peak (70+ weeks)",
                    "poultry_layer",
                    Some("poultry_layer_phase2"),
                    Some("layer_phase2"),
                    PresetAnimalParams {
                        age_weeks: Some(80),
                        production_pct: Some(70.0),
                        ..Default::default()
                    },
                    None,
                    &["layer_post_peak", "reduced_calcium"],
                ),
            ],
        },
    ]
}
