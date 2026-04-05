use serde::{Deserialize, Serialize};

use crate::db::feeds::Feed;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedGroup {
    Roughage,
    Succulent,
    Concentrate,
    Protein,
    AnimalOrigin,
    Mineral,
    Premix,
    Vitamin,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedSuitabilityStatus {
    Appropriate,
    Conditional,
    Restricted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSuitabilityAssessment {
    pub status: FeedSuitabilityStatus,
    pub notes: Vec<String>,
    pub max_inclusion_pct: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct TemplateShare {
    pub group: FeedGroup,
    pub share: f64,
    pub category: &'static str,
}

fn feed_search_text(feed: &Feed) -> String {
    let category = feed.category.to_lowercase();
    let subcategory = feed
        .subcategory
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    let name_ru = feed.name_ru.to_lowercase();
    let name_en = feed.name_en.as_deref().unwrap_or_default().to_lowercase();

    format!("{category} {subcategory} {name_ru} {name_en}")
}

fn push_unique_note(notes: &mut Vec<String>, note: &str) {
    if !notes.iter().any(|existing| existing == note) {
        notes.push(note.to_string());
    }
}

fn promote_suitability_status(current: &mut FeedSuitabilityStatus, next: FeedSuitabilityStatus) {
    match (*current, next) {
        (FeedSuitabilityStatus::Restricted, _) => {}
        (FeedSuitabilityStatus::Conditional, FeedSuitabilityStatus::Appropriate) => {}
        _ => *current = next,
    }
}

fn is_specialty_formula(feed: &Feed) -> bool {
    let search = feed_search_text(feed);
    matches!(classify_feed(feed), FeedGroup::Premix | FeedGroup::Vitamin)
        || matches!(feed.category.as_str(), "compound_feed" | "additive")
        || search.contains("premix")
        || search.contains("премикс")
        || search.contains("комбикорм")
        || search.contains("recipe")
        || search.contains("starter")
        || search.contains("grower")
        || search.contains("finisher")
        || search.contains("layer")
        || search.contains("broiler")
        || search.contains("несуш")
        || search.contains("бройл")
        || search.contains("свиномат")
        || search.contains("поросят")
}

fn is_layer_shell_grit(feed: &Feed) -> bool {
    let search = feed_search_text(feed);
    feed.subcategory.as_deref() == Some("layer_shell_grit")
        || search.contains("shell grit")
        || search.contains("ракушка")
}

fn is_potato_material(feed: &Feed) -> bool {
    let search = feed_search_text(feed);
    search.contains("potato") || search.contains("картоф")
}

fn is_raw_or_green_potato_material(feed: &Feed) -> bool {
    let search = feed_search_text(feed);
    is_potato_material(feed)
        && (search.contains("raw")
            || search.contains("сыр")
            || search.contains("haulm")
            || search.contains("tops")
            || search.contains("ботва"))
}

fn is_nonprotein_nitrogen(feed: &Feed) -> bool {
    let search = feed_search_text(feed);
    search.contains("urea")
        || search.contains("карбамид")
        || search.contains("мочевин")
        || search.contains("non protein nitrogen")
        || search.contains("nonprotein nitrogen")
        || search.contains("npn")
}

pub fn is_nonprotein_nitrogen_feed(feed: &Feed) -> bool {
    is_nonprotein_nitrogen(feed)
}

fn targets_non_supported_species(feed: &Feed) -> bool {
    let search = feed_search_text(feed);

    search.contains("овец")
        || search.contains("овц")
        || search.contains("sheep")
        || search.contains("lamb")
        || search.contains("goat")
        || search.contains("goats")
        || search.contains("коз")
        || search.contains("horse")
        || search.contains("horses")
        || search.contains("лошад")
        || search.contains("rabbit")
        || search.contains("rabbits")
        || search.contains("крол")
        || search.contains("fish")
        || search.contains("рыб")
        || search.contains("форел")
        || search.contains("carp")
        || search.contains("карп")
        || search.contains("fur")
        || search.contains("пушн")
        || search.contains("nutria")
        || search.contains("нутр")
        || search.contains("mink")
        || search.contains("норк")
}

fn stage_markers(feed: &Feed) -> (bool, bool, bool, bool, bool, bool) {
    let search = feed_search_text(feed);

    let dairy = search.contains("dairy")
        || search.contains("milk")
        || search.contains("lact")
        || search.contains("молоч")
        || search.contains("дойн")
        || search.contains("лактац");
    let beef = search.contains("beef")
        || search.contains("feedlot")
        || search.contains("fatten")
        || search.contains("откорм")
        || search.contains("мясн");
    let sow = search.contains("sow")
        || search.contains("gestat")
        || search.contains("breeding")
        || search.contains("свиномат")
        || search.contains("супорос")
        || search.contains("подсос");
    let swine_grower = search.contains("starter")
        || search.contains("grower")
        || search.contains("finisher")
        || search.contains("piglet")
        || search.contains("starter")
        || search.contains("поросят")
        || search.contains("стартер")
        || search.contains("гровер")
        || search.contains("финиш")
        || search.contains("откорм");
    let layer = search.contains("layer")
        || search.contains("laying")
        || search.contains("несуш")
        || search.contains("egg");
    let broiler = search.contains("broiler") || search.contains("бройлер");

    (dairy, beef, sow, swine_grower, layer, broiler)
}

#[derive(Debug, Clone, Copy, Default)]
struct CattleLifecycleMarkers {
    adult_cow: bool,
    lactating: bool,
    dry_cow: bool,
    calf: bool,
    youngstock: bool,
    beef: bool,
}

fn cattle_lifecycle_markers(feed: &Feed) -> CattleLifecycleMarkers {
    let search = feed_search_text(feed);

    let adult_cow = search.contains("cow")
        || search.contains("cows")
        || search.contains("коров")
        || search.contains("молоч")
        || search.contains("dairy")
        || search.contains("высокопродуктив")
        || search.contains("удой");
    let lactating = search.contains("lact")
        || search.contains("лактац")
        || search.contains("дойн")
        || search.contains("fresh")
        || search.contains("high-yield")
        || search.contains("высокопродуктив")
        || search.contains("удой");
    let dry_cow = search.contains("dry cow")
        || search.contains("dry cows")
        || search.contains("dry period")
        || search.contains("сухост")
        || search.contains("сухостой");
    let calf = search.contains("calf")
        || search.contains("calves")
        || search.contains("телят")
        || search.contains("телят")
        || search.contains("телен")
        || search.contains("до 6 мес")
        || search.contains("до 6-ти мес")
        || search.contains("1-6 мес")
        || search.contains("1-6 мес.")
        || search.contains("10 до 75 суток");
    let youngstock = search.contains("youngstock")
        || search.contains("heifer")
        || search.contains("heifers")
        || search.contains("ремонтн")
        || search.contains("телок")
        || search.contains("тёлок")
        || search.contains("молодняк")
        || search.contains("6-18 мес")
        || search.contains("6-12 мес")
        || search.contains("76 до 400 суток");
    let beef = search.contains("beef")
        || search.contains("feedlot")
        || search.contains("fatten")
        || search.contains("откорм")
        || search.contains("мясн");

    CattleLifecycleMarkers {
        adult_cow,
        lactating,
        dry_cow,
        calf,
        youngstock,
        beef,
    }
}

fn is_swine_nursery_formula(feed: &Feed) -> bool {
    let search = feed_search_text(feed);
    search.contains("piglet")
        || search.contains("nursery")
        || search.contains("starter")
        || search.contains("поросят")
        || search.contains("стартер")
}

fn is_swine_finisher_formula(feed: &Feed) -> bool {
    let search = feed_search_text(feed);
    search.contains("grower")
        || search.contains("finisher")
        || search.contains("откорм")
        || search.contains("гровер")
        || search.contains("финиш")
}

fn is_broiler_starter_formula(feed: &Feed) -> bool {
    let search = feed_search_text(feed);
    (search.contains("broiler") || search.contains("бройл"))
        && (search.contains("starter")
            || search.contains("1-4 нед")
            || search.contains("0-10")
            || search.contains("до 4 нед")
            || search.contains("1-3 нед"))
}

fn is_broiler_finisher_formula(feed: &Feed) -> bool {
    let search = feed_search_text(feed);
    (search.contains("broiler") || search.contains("бройл"))
        && (search.contains("finisher")
            || search.contains("старше 4 нед")
            || search.contains("4-8 нед")
            || search.contains("5-13 нед")
            || search.contains("25-42"))
}

#[derive(Debug, Clone, Copy, Default)]
struct StageContextFit {
    matches: bool,
    conflicts: bool,
    explicit_marker: bool,
}

fn cattle_stage_context_fit(feed: &Feed, stage_context: &str) -> Option<StageContextFit> {
    let context = stage_context.to_lowercase();
    if !context.contains("cattle") && context != "dairy" && context != "beef" {
        return None;
    }

    let markers = cattle_lifecycle_markers(feed);
    let explicit_marker = markers.adult_cow
        || markers.lactating
        || markers.dry_cow
        || markers.calf
        || markers.youngstock
        || markers.beef;
    if !explicit_marker {
        return Some(StageContextFit::default());
    }

    let (matches, conflicts) = if context.contains("cattle_dairy_dry") || context.contains("dry") {
        (
            markers.dry_cow,
            markers.calf || markers.youngstock || markers.beef || markers.lactating,
        )
    } else if context.contains("cattle_dairy") || context == "dairy" {
        (
            markers.adult_cow || markers.lactating,
            markers.calf || markers.youngstock || markers.dry_cow || markers.beef,
        )
    } else if context.contains("heifer")
        || context.contains("youngstock")
        || context.contains("replacement")
    {
        (
            markers.youngstock,
            markers.calf || markers.adult_cow || markers.lactating || markers.dry_cow,
        )
    } else if context.contains("cattle_beef") || context == "beef" {
        (
            markers.beef || markers.youngstock,
            markers.adult_cow || markers.lactating || markers.dry_cow || markers.calf,
        )
    } else {
        return None;
    };

    Some(StageContextFit {
        matches,
        conflicts,
        explicit_marker,
    })
}

fn stage_context_fit(feed: &Feed, stage_context: &str) -> StageContextFit {
    if let Some(cattle_fit) = cattle_stage_context_fit(feed, stage_context) {
        return cattle_fit;
    }

    let (dairy, beef, sow, swine_grower, layer, broiler) = stage_markers(feed);
    let swine_nursery = is_swine_nursery_formula(feed);
    let swine_finisher = is_swine_finisher_formula(feed);
    let broiler_starter = is_broiler_starter_formula(feed);
    let broiler_finisher = is_broiler_finisher_formula(feed);
    let explicit_marker = dairy
        || beef
        || sow
        || swine_grower
        || swine_nursery
        || swine_finisher
        || layer
        || broiler
        || broiler_starter
        || broiler_finisher;
    if !explicit_marker {
        return StageContextFit::default();
    }

    let context = stage_context.to_lowercase();
    let (matches_stage, conflicts_stage) = if context.contains("cattle_dairy") || context == "dairy"
    {
        (dairy, beef)
    } else if context.contains("cattle_beef") || context == "beef" {
        (beef, dairy)
    } else if context.contains("swine_sow")
        || context.contains("gestation")
        || context.contains("lactating")
    {
        (sow, swine_nursery || swine_finisher)
    } else if context.contains("swine") {
        if context.contains("piglet") || context.contains("nursery") || context.contains("starter")
        {
            (
                swine_nursery || (swine_grower && !swine_finisher),
                sow || swine_finisher,
            )
        } else if context.contains("finisher") || context.contains("grower") {
            (swine_finisher || swine_grower, sow || swine_nursery)
        } else {
            (swine_grower || swine_nursery || swine_finisher, sow)
        }
    } else if context.contains("poultry_layer") || context == "layer" {
        (layer, broiler || broiler_starter || broiler_finisher)
    } else if context.contains("poultry_broiler") || context.contains("broiler") {
        if context.contains("starter") {
            (
                broiler_starter || (broiler && !broiler_finisher),
                layer || broiler_finisher,
            )
        } else if context.contains("finisher") {
            (
                broiler_finisher || (broiler && !broiler_starter),
                layer || broiler_starter,
            )
        } else {
            (broiler || broiler_starter || broiler_finisher, layer)
        }
    } else {
        (false, false)
    };

    StageContextFit {
        matches: matches_stage,
        conflicts: conflicts_stage,
        explicit_marker,
    }
}

pub fn classify_feed(feed: &Feed) -> FeedGroup {
    let category = feed.category.to_lowercase();
    let search = feed_search_text(feed);

    if search.contains("premix") || search.contains("премикс") {
        return FeedGroup::Premix;
    }
    if search.contains("chalk")
        || search.contains("limestone")
        || search.contains("monocalcium")
        || search.contains("dicalcium")
        || search.contains("phosphate")
        || search.contains("salt")
        || search.contains("shell")
        || search.contains("ракуш")
        || search.contains("мел")
        || search.contains("соль")
        || search.contains("фосфат")
        || search.contains("зола")
    {
        return FeedGroup::Mineral;
    }
    if search.contains("fish meal")
        || search.contains("meat")
        || search.contains("bone meal")
        || search.contains("blood meal")
        || search.contains("рыбн")
        || search.contains("мясо")
        || search.contains("костн")
    {
        return FeedGroup::AnimalOrigin;
    }
    if search.contains("soy")
        || search.contains("rape")
        || search.contains("sunflower meal")
        || search.contains("cottonseed")
        || search.contains("meal")
        || search.contains("cake")
        || search.contains("шрот")
        || search.contains("жмых")
        || search.contains("соев")
        || search.contains("подсолнеч")
    {
        return FeedGroup::Protein;
    }
    if category == "roughage"
        || search.contains("hay")
        || search.contains("straw")
        || search.contains("haylage")
        || search.contains("сено")
        || search.contains("солом")
        || search.contains("сенаж")
    {
        return FeedGroup::Roughage;
    }
    if category == "silage"
        || category == "succulent"
        || search.contains("silage")
        || search.contains("green")
        || search.contains("beet")
        || search.contains("root")
        || search.contains("сило")
        || search.contains("корнеплод")
        || search.contains("трава")
    {
        return FeedGroup::Succulent;
    }
    if category == "grain"
        || category == "concentrate"
        || search.contains("barley")
        || search.contains("corn")
        || search.contains("wheat")
        || search.contains("oat")
        || search.contains("bran")
        || search.contains("ячм")
        || search.contains("пшен")
        || search.contains("кукуруз")
        || search.contains("овес")
        || search.contains("отруб")
    {
        return FeedGroup::Concentrate;
    }
    if search.contains("vitamin") || search.contains("витамин") {
        return FeedGroup::Vitamin;
    }

    let crude_fiber = feed.crude_fiber.unwrap_or(0.0);
    let crude_protein = feed.crude_protein.unwrap_or(0.0);
    let calcium = feed.calcium.unwrap_or(0.0);
    let phosphorus = feed.phosphorus.unwrap_or(0.0);

    if calcium > 150.0 || phosphorus > 80.0 {
        return FeedGroup::Mineral;
    }
    if crude_protein >= 250.0 {
        return FeedGroup::Protein;
    }
    if crude_fiber >= 180.0 {
        return FeedGroup::Roughage;
    }

    FeedGroup::Other
}

fn species_markers(feed: &Feed) -> (bool, bool, bool) {
    let search = feed_search_text(feed);

    let cattle = search.contains("крс")
        || search.contains("коров")
        || search.contains("телят")
        || search.contains("cattle")
        || search.contains("bovine")
        || search.contains("dairy");
    let swine = search.contains("свин")
        || search.contains("pig")
        || search.contains("swine")
        || search.contains("porcine")
        || search.contains("sow");
    let poultry = search.contains("птиц")
        || search.contains("куры")
        || search.contains("куриц")
        || search.contains("несуш")
        || search.contains("бройл")
        || search.contains("chicken")
        || search.contains("broiler")
        || search.contains("layer")
        || search.contains("poultry");

    (cattle, swine, poultry)
}

fn species_max_inclusion_pct(feed: &Feed, species: &str) -> Option<f64> {
    let raw = match species {
        "swine" => feed.max_inclusion_pig,
        "poultry" => feed.max_inclusion_poultry,
        _ => feed.max_inclusion_cattle,
    }?;

    if raw <= 0.0 {
        None
    } else {
        Some((raw * 10.0).round() / 10.0)
    }
}

pub fn assess_feed_suitability(
    feed: &Feed,
    species: &str,
    stage_context: Option<&str>,
) -> FeedSuitabilityAssessment {
    const NOTE_SPECIES_MATCH: &str = "Species-targeted formula matches the current animal type.";
    const NOTE_SPECIES_MISMATCH: &str =
        "Species-targeted formula does not match the current animal type.";
    const NOTE_STAGE_MATCH: &str = "Stage-targeted formula matches the current production phase.";
    const NOTE_STAGE_MISMATCH: &str =
        "Stage-targeted formula does not match the current production phase.";
    const NOTE_STAGE_CONFIRM: &str =
        "Stage-targeted formula requires production-phase confirmation.";
    const NOTE_CATTLE_CLASS_MATCH: &str =
        "Cattle-targeted formula matches the current cattle class.";
    const NOTE_CATTLE_CLASS_MISMATCH: &str =
        "Cattle-targeted formula does not match the current cattle class.";
    const NOTE_CATTLE_CLASS_CONFIRM: &str =
        "Cattle-targeted formula requires cattle-class confirmation.";
    const NOTE_LAYER_SHELL: &str = "Layer shell grit is intended for egg-producing poultry.";
    const NOTE_RAW_POTATO_POULTRY: &str =
        "Raw potato ingredients are excluded from the poultry candidate set.";
    const NOTE_POTATO_CONDITIONAL: &str =
        "Potato ingredients require processing-state and inclusion review before use.";
    const NOTE_NPN_RESTRICTED: &str =
        "Non-protein nitrogen sources such as urea are not offered for swine or poultry rations.";
    const NOTE_NPN_CONDITIONAL: &str =
        "Non-protein nitrogen sources such as urea require acclimated ruminants, controlled inclusion, and adequate fermentable energy.";
    const NOTE_NON_SUPPORTED_SPECIES: &str =
        "Formula targets a species that is outside the current cattle, swine, and poultry scope.";
    const NOTE_MAX_INCLUSION: &str = "Keep within the species-specific inclusion limit.";

    let species = species.trim().to_lowercase();
    let stage_context = stage_context.unwrap_or_default();
    let specialty_formula = is_specialty_formula(feed);
    let stage_fit = stage_context_fit(feed, stage_context);
    let (cattle, swine, poultry) = species_markers(feed);
    let explicit_species_marker = cattle || swine || poultry;
    let species_matches = match species.as_str() {
        "cattle" => cattle,
        "swine" => swine,
        "poultry" => poultry,
        _ => false,
    };

    let mut assessment = FeedSuitabilityAssessment {
        status: FeedSuitabilityStatus::Appropriate,
        notes: Vec::new(),
        max_inclusion_pct: species_max_inclusion_pct(feed, species.as_str()),
    };

    if explicit_species_marker {
        if species_matches {
            push_unique_note(&mut assessment.notes, NOTE_SPECIES_MATCH);
        } else {
            promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Restricted);
            push_unique_note(&mut assessment.notes, NOTE_SPECIES_MISMATCH);
        }
    }

    if targets_non_supported_species(feed) {
        promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Restricted);
        push_unique_note(&mut assessment.notes, NOTE_NON_SUPPORTED_SPECIES);
    }

    if specialty_formula && stage_fit.explicit_marker {
        let (match_note, mismatch_note, confirm_note) = if species == "cattle" {
            (
                NOTE_CATTLE_CLASS_MATCH,
                NOTE_CATTLE_CLASS_MISMATCH,
                NOTE_CATTLE_CLASS_CONFIRM,
            )
        } else {
            (NOTE_STAGE_MATCH, NOTE_STAGE_MISMATCH, NOTE_STAGE_CONFIRM)
        };
        if stage_fit.matches {
            push_unique_note(&mut assessment.notes, match_note);
        } else if stage_fit.conflicts {
            promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Restricted);
            push_unique_note(&mut assessment.notes, mismatch_note);
        } else if species == "cattle" {
            promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Conditional);
            push_unique_note(&mut assessment.notes, confirm_note);
        } else if stage_context.trim().is_empty() {
            promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Conditional);
            push_unique_note(&mut assessment.notes, confirm_note);
        }
    }

    if is_layer_shell_grit(feed) {
        if species != "poultry" || stage_context.to_lowercase().contains("broiler") {
            promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Restricted);
        } else if !stage_context.to_lowercase().contains("layer") {
            promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Conditional);
        }
        push_unique_note(&mut assessment.notes, NOTE_LAYER_SHELL);
    }

    if is_raw_or_green_potato_material(feed) {
        if species == "poultry" {
            promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Restricted);
            push_unique_note(&mut assessment.notes, NOTE_RAW_POTATO_POULTRY);
        } else {
            promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Conditional);
            push_unique_note(&mut assessment.notes, NOTE_POTATO_CONDITIONAL);
        }
    } else if is_potato_material(feed) && species == "poultry" {
        promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Conditional);
        push_unique_note(&mut assessment.notes, NOTE_POTATO_CONDITIONAL);
    }

    if is_nonprotein_nitrogen(feed) {
        if species == "cattle" {
            promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Conditional);
            push_unique_note(&mut assessment.notes, NOTE_NPN_CONDITIONAL);
        } else {
            promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Restricted);
            push_unique_note(&mut assessment.notes, NOTE_NPN_RESTRICTED);
        }
    }

    if let Some(max_inclusion_pct) = assessment.max_inclusion_pct {
        if max_inclusion_pct <= 25.0
            || matches!(
                classify_feed(feed),
                FeedGroup::Premix | FeedGroup::Vitamin | FeedGroup::Mineral
            )
        {
            promote_suitability_status(&mut assessment.status, FeedSuitabilityStatus::Conditional);
        }
        push_unique_note(&mut assessment.notes, NOTE_MAX_INCLUSION);
    }

    assessment
}

pub fn is_feed_allowed_for_context(
    feed: &Feed,
    species: &str,
    stage_context: Option<&str>,
) -> bool {
    assess_feed_suitability(feed, species, stage_context).status
        != FeedSuitabilityStatus::Restricted
}

pub fn is_feed_species_appropriate(feed: &Feed, species: &str) -> bool {
    is_feed_allowed_for_context(feed, species, None)
}

pub fn species_fit_bonus(feed: &Feed, species: &str) -> f64 {
    let (cattle, swine, poultry) = species_markers(feed);
    if !cattle && !swine && !poultry {
        return 0.0;
    }

    match species {
        "cattle" if cattle => 15.0,
        "swine" if swine => 15.0,
        "poultry" if poultry => 15.0,
        _ => -200.0,
    }
}

pub fn stage_fit_bonus(feed: &Feed, stage_context: &str) -> f64 {
    let fit = stage_context_fit(feed, stage_context);
    if !fit.explicit_marker {
        return 0.0;
    }

    if fit.matches {
        18.0
    } else if fit.conflicts {
        -90.0
    } else {
        0.0
    }
}

pub fn group_label(group: FeedGroup) -> &'static str {
    match group {
        FeedGroup::Roughage => "roughage",
        FeedGroup::Succulent => "succulent",
        FeedGroup::Concentrate => "concentrate",
        FeedGroup::Protein => "protein",
        FeedGroup::AnimalOrigin => "animal_origin",
        FeedGroup::Mineral => "mineral",
        FeedGroup::Premix => "premix",
        FeedGroup::Vitamin => "vitamin",
        FeedGroup::Other => "other",
    }
}

fn is_silage_like(feed: &Feed) -> bool {
    let category = feed.category.trim().to_ascii_lowercase();
    let search = feed_search_text(feed);
    category == "silage"
        || search.contains("silage")
        || search.contains("haylage")
        || search.contains("сенаж")
        || search.contains("силос")
}

pub fn matches_runtime_category(feed: &Feed, category: &str) -> bool {
    let db_category = feed.category.trim().to_ascii_lowercase();

    match category {
        "roughage" => {
            db_category == "roughage"
                || (classify_feed(feed) == FeedGroup::Roughage && !is_silage_like(feed))
        }
        "silage" => is_silage_like(feed),
        "succulent" => {
            db_category == "succulent"
                || (classify_feed(feed) == FeedGroup::Succulent && !is_silage_like(feed))
        }
        "concentrate" => {
            matches!(db_category.as_str(), "grain" | "concentrate")
                || classify_feed(feed) == FeedGroup::Concentrate
        }
        "protein" => {
            matches!(db_category.as_str(), "protein" | "oilseed_meal")
                || classify_feed(feed) == FeedGroup::Protein
        }
        "animal_origin" => {
            db_category == "animal_origin" || classify_feed(feed) == FeedGroup::AnimalOrigin
        }
        "mineral" => db_category == "mineral" || classify_feed(feed) == FeedGroup::Mineral,
        "premix" => {
            db_category == "premix"
                || matches!(classify_feed(feed), FeedGroup::Premix | FeedGroup::Vitamin)
        }
        "additive" => db_category == "additive",
        "npn" => is_nonprotein_nitrogen(feed),
        "other" => classify_feed(feed) == FeedGroup::Other,
        _ => false,
    }
}

pub fn matrix_group_for_category(category: &str) -> Option<FeedGroup> {
    match category {
        "roughage" => Some(FeedGroup::Roughage),
        "silage" | "succulent" => Some(FeedGroup::Succulent),
        "concentrate" => Some(FeedGroup::Concentrate),
        "protein" => Some(FeedGroup::Protein),
        "animal_origin" => Some(FeedGroup::AnimalOrigin),
        "mineral" => Some(FeedGroup::Mineral),
        "premix" => Some(FeedGroup::Premix),
        _ => None,
    }
}

pub fn template_for_group(group_id: Option<&str>, species: &str) -> Vec<TemplateShare> {
    let group_id = group_id.unwrap_or_default();

    if group_id.contains("beef") {
        // Beef finishing: 60-70% concentrate, 15-20% forage (NRC/NASEM, OK State Extension)
        return vec![
            TemplateShare {
                group: FeedGroup::Roughage,
                share: 0.08,
                category: "roughage",
            },
            TemplateShare {
                group: FeedGroup::Succulent,
                share: 0.10,
                category: "silage",
            },
            TemplateShare {
                group: FeedGroup::Concentrate,
                share: 0.60,
                category: "concentrate",
            },
            TemplateShare {
                group: FeedGroup::Protein,
                share: 0.12,
                category: "protein",
            },
            TemplateShare {
                group: FeedGroup::Mineral,
                share: 0.03,
                category: "mineral",
            },
            TemplateShare {
                group: FeedGroup::Premix,
                share: 0.02,
                category: "premix",
            },
            TemplateShare {
                group: FeedGroup::Other,
                share: 0.01,
                category: "additive",
            },
        ];
    }
    if group_id.contains("sow") {
        // Gestating sow: 70% concentrate, 14% protein, moderate supplements (NRC 2012, K-State)
        return vec![
            TemplateShare {
                group: FeedGroup::Succulent,
                share: 0.10,
                category: "succulent",
            },
            TemplateShare {
                group: FeedGroup::Concentrate,
                share: 0.70,
                category: "concentrate",
            },
            TemplateShare {
                group: FeedGroup::Protein,
                share: 0.14,
                category: "protein",
            },
            TemplateShare {
                group: FeedGroup::Mineral,
                share: 0.03,
                category: "mineral",
            },
            TemplateShare {
                group: FeedGroup::Premix,
                share: 0.02,
                category: "premix",
            },
            TemplateShare {
                group: FeedGroup::Other,
                share: 0.01,
                category: "additive",
            },
        ];
    }
    if group_id.contains("swine") {
        // Swine finisher: 82% concentrate, 14% protein (NRC 2012, UMN Extension)
        return vec![
            TemplateShare {
                group: FeedGroup::Concentrate,
                share: 0.82,
                category: "concentrate",
            },
            TemplateShare {
                group: FeedGroup::Protein,
                share: 0.14,
                category: "protein",
            },
            TemplateShare {
                group: FeedGroup::Mineral,
                share: 0.02,
                category: "mineral",
            },
            TemplateShare {
                group: FeedGroup::Premix,
                share: 0.01,
                category: "premix",
            },
            TemplateShare {
                group: FeedGroup::Other,
                share: 0.01,
                category: "additive",
            },
        ];
    }
    if group_id.contains("layer") {
        // Layer: 62% concentrate, 20% protein, 10% mineral for Ca (VNITIP, Feed Strategy)
        return vec![
            TemplateShare {
                group: FeedGroup::Concentrate,
                share: 0.62,
                category: "concentrate",
            },
            TemplateShare {
                group: FeedGroup::Protein,
                share: 0.20,
                category: "protein",
            },
            TemplateShare {
                group: FeedGroup::Mineral,
                share: 0.10,
                category: "mineral",
            },
            TemplateShare {
                group: FeedGroup::Premix,
                share: 0.02,
                category: "premix",
            },
            TemplateShare {
                group: FeedGroup::AnimalOrigin,
                share: 0.03,
                category: "animal_origin",
            },
            TemplateShare {
                group: FeedGroup::Other,
                share: 0.01,
                category: "additive",
            },
        ];
    }
    if group_id.contains("broiler") || species == "poultry" {
        return vec![
            TemplateShare {
                group: FeedGroup::Concentrate,
                share: 0.68,
                category: "concentrate",
            },
            TemplateShare {
                group: FeedGroup::Protein,
                share: 0.20,
                category: "protein",
            },
            TemplateShare {
                group: FeedGroup::AnimalOrigin,
                share: 0.04,
                category: "animal_origin",
            },
            TemplateShare {
                group: FeedGroup::Mineral,
                share: 0.04,
                category: "mineral",
            },
            TemplateShare {
                group: FeedGroup::Premix,
                share: 0.02,
                category: "premix",
            },
            TemplateShare {
                group: FeedGroup::Other,
                share: 0.02,
                category: "additive",
            },
        ];
    }

    // Dairy cow early lactation: 45-50% forage, 35-40% concentrate (NRC 2021, Penn State)
    vec![
        TemplateShare {
            group: FeedGroup::Roughage,
            share: 0.18,
            category: "roughage",
        },
        TemplateShare {
            group: FeedGroup::Succulent,
            share: 0.25,
            category: "silage",
        },
        TemplateShare {
            group: FeedGroup::Succulent,
            share: 0.07,
            category: "succulent",
        },
        TemplateShare {
            group: FeedGroup::Concentrate,
            share: 0.35,
            category: "concentrate",
        },
        TemplateShare {
            group: FeedGroup::Protein,
            share: 0.10,
            category: "protein",
        },
        TemplateShare {
            group: FeedGroup::Mineral,
            share: 0.02,
            category: "mineral",
        },
        TemplateShare {
            group: FeedGroup::Premix,
            share: 0.02,
            category: "premix",
        },
        TemplateShare {
            group: FeedGroup::Other,
            share: 0.01,
            category: "additive",
        },
    ]
}

pub fn energy_density(feed: &Feed, species: &str) -> f64 {
    let dm_share = feed.dry_matter.unwrap_or(86.0) / 100.0;
    match species {
        "swine" => feed.energy_oe_pig.unwrap_or(0.0) * dm_share,
        "poultry" => feed.energy_oe_poultry.unwrap_or(0.0),
        _ => feed.energy_oe_cattle.unwrap_or(0.0) * dm_share,
    }
}

pub fn protein_density(feed: &Feed) -> f64 {
    feed.crude_protein.unwrap_or(0.0) + feed.lysine.unwrap_or(0.0) * 8.0
}

pub fn mineral_density(feed: &Feed) -> f64 {
    let macro_minerals = feed.calcium.unwrap_or(0.0) * 2.0 + feed.phosphorus.unwrap_or(0.0);
    // Small trace mineral bonus — never dominate over Ca/P scoring
    let trace_bonus = (feed.zinc.unwrap_or(0.0) * 0.02
        + feed.iron.unwrap_or(0.0) * 0.01
        + feed.manganese.unwrap_or(0.0) * 0.01
        + feed.copper.unwrap_or(0.0) * 0.05
        + feed.cobalt.unwrap_or(0.0) * 0.5)
        .min(50.0); // cap at 50 so macro minerals always dominate
    macro_minerals + trace_bonus
}

pub fn vitamin_density(feed: &Feed) -> f64 {
    feed.carotene.unwrap_or(0.0) / 10.0
        + feed.vit_d3.unwrap_or(0.0) / 200.0
        + feed.vit_e.unwrap_or(0.0)
}

pub fn score_feed_for_group(feed: &Feed, group: FeedGroup, species: &str) -> f64 {
    let verified_bonus = if feed.verified { 25.0 } else { 0.0 };
    let price_bonus = if feed.price_per_ton.unwrap_or(0.0) > 0.0 {
        10.0
    } else {
        0.0
    };
    let species_bonus = species_fit_bonus(feed, species);
    let commonness = commonness_bonus(feed, group);

    (match group {
        FeedGroup::Roughage => energy_density(feed, species) * 8.0 + feed.crude_fiber.unwrap_or(0.0) * 0.02,
        FeedGroup::Succulent => {
            energy_density(feed, species) * 10.0 + (100.0 - feed.dry_matter.unwrap_or(86.0))
        }
        FeedGroup::Concentrate => {
            energy_density(feed, species) * 14.0 + feed.starch.unwrap_or(0.0) * 0.02
        }
        FeedGroup::Protein => protein_density(feed) * 0.5 + energy_density(feed, species) * 4.0,
        FeedGroup::AnimalOrigin => protein_density(feed) * 0.55,
        FeedGroup::Mineral => mineral_density(feed),
        FeedGroup::Premix | FeedGroup::Vitamin => vitamin_density(feed),
        FeedGroup::Other => energy_density(feed, species),
    }) + verified_bonus
        + price_bonus
        + species_bonus
        + commonness
}

/// Bonus for commonly used, widely available feeds.
/// Standard farm feeds (barley, wheat, corn, soybean meal, hay, etc.)
/// get a boost to prevent exotic/niche feeds from dominating auto-populate.
fn commonness_bonus(feed: &Feed, group: FeedGroup) -> f64 {
    let name = feed.name_ru.to_lowercase();
    let bonus = match group {
        FeedGroup::Concentrate => {
            if name.contains("ячмень") || name.contains("barley") {
                30.0
            } else if name.contains("пшеница") || name.contains("wheat") {
                28.0
            } else if name.contains("кукуруз") || name.contains("corn") || name.contains("маис") {
                28.0
            } else if name.contains("овёс") || name.contains("овес") || name.contains("oat") {
                22.0
            } else if name.contains("отруби") || name.contains("bran") {
                15.0
            } else {
                0.0
            }
        }
        FeedGroup::Roughage => {
            if name.contains("сено") || name.contains("hay") {
                25.0
            } else if name.contains("сенаж") || name.contains("haylage") {
                20.0
            } else if name.contains("солома") || name.contains("straw") {
                10.0
            } else {
                0.0
            }
        }
        FeedGroup::Succulent => {
            if name.contains("силос") || name.contains("silage") {
                25.0
            } else if name.contains("свёкл") || name.contains("свекл") || name.contains("beet") {
                20.0
            } else {
                0.0
            }
        }
        FeedGroup::Protein => {
            if name.contains("соев") || name.contains("soybean") || name.contains("soy") {
                25.0
            } else if name.contains("подсолнечн") || name.contains("sunflower") {
                22.0
            } else if name.contains("рапс") || name.contains("rapeseed") || name.contains("canola") {
                18.0
            } else {
                0.0
            }
        }
        FeedGroup::Mineral => {
            if name.contains("мел") || name.contains("chalk") || name.contains("известняк") || name.contains("limestone") {
                20.0
            } else if name.contains("фосфат") || name.contains("phosphate") {
                18.0
            } else if name.contains("соль") || name.contains("salt") {
                15.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    };

    // Penalize feeds with very long names (often specialized/niche formulas)
    let name_penalty = if feed.name_ru.len() > 80 { -10.0 } else { 0.0 };

    bonus + name_penalty
}

pub fn preferred_groups_for_nutrient(key: &str, species: &str) -> &'static [FeedGroup] {
    match key {
        "energy_eke" | "energy_oe_cattle" => &[
            FeedGroup::Concentrate,
            FeedGroup::Succulent,
            FeedGroup::Roughage,
        ],
        "energy_oe_pig" | "energy_oe_poultry" => &[FeedGroup::Concentrate, FeedGroup::Protein],
        "crude_protein"
        | "crude_protein_pct"
        | "lysine"
        | "lysine_sid"
        | "lysine_sid_pct"
        | "lysine_tid_pct"
        | "methionine_cystine"
        | "methionine_cystine_sid"
        | "methionine_cystine_tid_pct" => &[
            FeedGroup::Protein,
            FeedGroup::AnimalOrigin,
            FeedGroup::Concentrate,
        ],
        "crude_fiber" => &[FeedGroup::Roughage, FeedGroup::Succulent],
        "calcium" | "calcium_pct" | "phosphorus" => {
            &[FeedGroup::Mineral, FeedGroup::Premix]
        }
        "carotene" | "vit_d3" | "vit_e" => {
            &[FeedGroup::Premix, FeedGroup::Vitamin, FeedGroup::Mineral]
        }
        _ if matches!(
            key,
            "magnesium"
                | "potassium"
                | "sodium"
                | "sulfur"
                | "iron"
                | "copper"
                | "zinc"
                | "manganese"
                | "cobalt"
                | "iodine"
        ) =>
        {
            &[FeedGroup::Mineral, FeedGroup::Premix, FeedGroup::Concentrate]
        }
        _ if species == "cattle" => &[FeedGroup::Concentrate, FeedGroup::Roughage],
        _ => &[FeedGroup::Concentrate, FeedGroup::Protein],
    }
}

/// Returns the mandatory feed groups for a given species and optional animal group context.
///
/// Cattle always need Roughage + Succulent + Concentrate + Protein + Mineral.
/// Dairy/fresh/lactating cattle additionally require Premix.
/// Swine and poultry need Concentrate + Protein + Mineral + Premix.
/// Fallback: extract unique groups from `template_for_group()`.
pub fn required_groups_for_species(species: &str, animal_group_id: Option<&str>) -> Vec<FeedGroup> {
    let group_id = animal_group_id.unwrap_or_default().to_lowercase();

    match species {
        "cattle" => {
            let mut groups = vec![
                FeedGroup::Roughage,
                FeedGroup::Succulent,
                FeedGroup::Concentrate,
                FeedGroup::Protein,
                FeedGroup::Mineral,
            ];
            if group_id.contains("dairy")
                || group_id.contains("fresh")
                || group_id.contains("lact")
            {
                groups.push(FeedGroup::Premix);
            }
            groups
        }
        "swine" => vec![
            FeedGroup::Concentrate,
            FeedGroup::Protein,
            FeedGroup::Mineral,
            FeedGroup::Premix,
        ],
        "poultry" => vec![
            FeedGroup::Concentrate,
            FeedGroup::Protein,
            FeedGroup::Mineral,
            FeedGroup::Premix,
        ],
        _ => {
            let template = template_for_group(animal_group_id, species);
            let mut seen = std::collections::HashSet::new();
            template
                .into_iter()
                .filter_map(|ts| {
                    if seen.insert(ts.group) {
                        Some(ts.group)
                    } else {
                        None
                    }
                })
                .collect()
        }
    }
}

/// Returns the feed groups that are required but not present in the given set.
pub fn validate_group_coverage(
    present_groups: &[FeedGroup],
    required_groups: &[FeedGroup],
) -> Vec<FeedGroup> {
    let present: std::collections::HashSet<FeedGroup> = present_groups.iter().copied().collect();
    required_groups
        .iter()
        .filter(|g| !present.contains(g))
        .copied()
        .collect()
}

/// Groups feeds by their `FeedGroup` classification.
pub fn feeds_by_group(feeds: &[Feed]) -> std::collections::HashMap<FeedGroup, Vec<&Feed>> {
    let mut grouped: std::collections::HashMap<FeedGroup, Vec<&Feed>> =
        std::collections::HashMap::new();
    for feed in feeds {
        let group = classify_feed(feed);
        grouped.entry(group).or_default().push(feed);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_groups_cover_all_livestock_types() {
        let cattle_base = required_groups_for_species("cattle", Some("cattle_beef_500"));
        assert!(cattle_base.contains(&FeedGroup::Roughage));
        assert!(cattle_base.contains(&FeedGroup::Succulent));
        assert!(cattle_base.contains(&FeedGroup::Concentrate));
        assert!(cattle_base.contains(&FeedGroup::Protein));
        assert!(cattle_base.contains(&FeedGroup::Mineral));
        assert!(!cattle_base.contains(&FeedGroup::Premix)); // beef doesn't need premix

        let cattle_dairy = required_groups_for_species("cattle", Some("cattle_dairy_25"));
        assert!(cattle_dairy.contains(&FeedGroup::Premix));
        assert!(cattle_dairy.len() >= 6);

        let swine = required_groups_for_species("swine", Some("swine_finisher"));
        assert!(swine.contains(&FeedGroup::Concentrate));
        assert!(swine.contains(&FeedGroup::Protein));
        assert!(swine.contains(&FeedGroup::Mineral));
        assert!(swine.contains(&FeedGroup::Premix));

        let poultry = required_groups_for_species("poultry", Some("poultry_broiler"));
        assert!(poultry.contains(&FeedGroup::Concentrate));
        assert!(poultry.contains(&FeedGroup::Protein));
        assert!(poultry.contains(&FeedGroup::Mineral));
        assert!(poultry.contains(&FeedGroup::Premix));

        // Fallback for unknown species should extract from template
        let fallback = required_groups_for_species("horse", None);
        assert!(!fallback.is_empty());
    }

    #[test]
    fn validate_group_coverage_detects_missing_groups() {
        let required = vec![
            FeedGroup::Roughage,
            FeedGroup::Succulent,
            FeedGroup::Concentrate,
            FeedGroup::Protein,
            FeedGroup::Mineral,
        ];
        let present = vec![FeedGroup::Roughage, FeedGroup::Concentrate];
        let missing = validate_group_coverage(&present, &required);
        assert_eq!(missing.len(), 3);
        assert!(missing.contains(&FeedGroup::Succulent));
        assert!(missing.contains(&FeedGroup::Protein));
        assert!(missing.contains(&FeedGroup::Mineral));
    }

    #[test]
    fn validate_group_coverage_passes_when_complete() {
        let required = vec![
            FeedGroup::Concentrate,
            FeedGroup::Protein,
            FeedGroup::Mineral,
            FeedGroup::Premix,
        ];
        let present = vec![
            FeedGroup::Concentrate,
            FeedGroup::Protein,
            FeedGroup::Mineral,
            FeedGroup::Premix,
            FeedGroup::Other,
        ];
        let missing = validate_group_coverage(&present, &required);
        assert!(missing.is_empty());
    }

    #[test]
    fn feeds_by_group_classifies_correctly() {
        let feeds = vec![
            Feed {
                name_ru: "Hay".to_string(),
                category: "roughage".to_string(),
                ..Default::default()
            },
            Feed {
                name_ru: "Barley grain".to_string(),
                category: "grain".to_string(),
                ..Default::default()
            },
            Feed {
                name_ru: "Soybean meal".to_string(),
                category: "oilseed_meal".to_string(),
                crude_protein: Some(430.0),
                ..Default::default()
            },
            Feed {
                name_ru: "Feed chalk".to_string(),
                category: "mineral".to_string(),
                calcium: Some(360.0),
                ..Default::default()
            },
        ];
        let grouped = feeds_by_group(&feeds);
        assert!(grouped.contains_key(&FeedGroup::Roughage));
        assert!(grouped.contains_key(&FeedGroup::Concentrate));
        assert!(grouped.contains_key(&FeedGroup::Protein));
        assert!(grouped.contains_key(&FeedGroup::Mineral));
        assert_eq!(grouped[&FeedGroup::Roughage].len(), 1);
    }

    #[test]
    fn classifies_mineral_feed() {
        let feed = Feed {
            name_ru: "Feed chalk".to_string(),
            category: "other".to_string(),
            calcium: Some(360.0),
            ..Default::default()
        };
        assert_eq!(classify_feed(&feed), FeedGroup::Mineral);
    }

    #[test]
    fn classifies_protein_feed() {
        let feed = Feed {
            name_ru: "Soybean meal".to_string(),
            category: "oilseed_meal".to_string(),
            crude_protein: Some(430.0),
            ..Default::default()
        };
        assert_eq!(classify_feed(&feed), FeedGroup::Protein);
    }

    #[test]
    fn detects_species_specific_premix() {
        let cattle_premix = Feed {
            name_ru: "Премикс П60-1 для КРС".to_string(),
            category: "premix".to_string(),
            ..Default::default()
        };
        let layer_premix = Feed {
            name_ru: "Премикс для кур-несушек".to_string(),
            category: "premix".to_string(),
            ..Default::default()
        };

        assert!(is_feed_species_appropriate(&cattle_premix, "cattle"));
        assert!(!is_feed_species_appropriate(&cattle_premix, "poultry"));
        assert!(is_feed_species_appropriate(&layer_premix, "poultry"));
        assert!(!is_feed_species_appropriate(&layer_premix, "cattle"));
    }

    #[test]
    fn stage_fit_bonus_prefers_matching_phase() {
        let sow_feed = Feed {
            name_ru: "Премикс для свиноматок".to_string(),
            category: "premix".to_string(),
            ..Default::default()
        };
        let starter_feed = Feed {
            name_ru: "Премикс стартер для поросят".to_string(),
            category: "premix".to_string(),
            ..Default::default()
        };

        assert!(stage_fit_bonus(&sow_feed, "swine_sow_lactating") > 0.0);
        assert!(stage_fit_bonus(&starter_feed, "swine_sow_lactating") < 0.0);
    }

    #[test]
    fn restricts_calf_premix_for_lactating_dairy_context() {
        let calf_premix = Feed {
            name_ru:
                "Рецепты премиксов для молодняка крупного рогатого скота, на 1 тонну До 6-ти мес. возраста П 62-3-89"
                    .to_string(),
            category: "premix".to_string(),
            ..Default::default()
        };

        let assessment =
            assess_feed_suitability(&calf_premix, "cattle", Some("cattle_dairy_fresh"));

        assert_eq!(assessment.status, FeedSuitabilityStatus::Restricted);
        assert!(assessment
            .notes
            .iter()
            .any(|note| note.contains("current cattle class")));
    }

    #[test]
    fn keeps_generic_cow_premix_conditional_for_dry_cow_context() {
        let adult_cow_premix = Feed {
            name_ru: "Премикс для молочных коров".to_string(),
            category: "premix".to_string(),
            ..Default::default()
        };

        let assessment =
            assess_feed_suitability(&adult_cow_premix, "cattle", Some("cattle_dairy_dry_early"));

        assert_eq!(assessment.status, FeedSuitabilityStatus::Conditional);
        assert!(assessment
            .notes
            .iter()
            .any(|note| note.contains("cattle-class")));
    }

    #[test]
    fn assesses_layer_shell_grit_as_restricted_for_broilers() {
        let shell_grit = Feed {
            name_ru: "Ракушка кормовая для несушек".to_string(),
            category: "mineral".to_string(),
            subcategory: Some("layer_shell_grit".to_string()),
            ..Default::default()
        };

        let assessment = assess_feed_suitability(&shell_grit, "poultry", Some("poultry_broiler"));

        assert_eq!(assessment.status, FeedSuitabilityStatus::Restricted);
        assert!(assessment
            .notes
            .iter()
            .any(|note| note.contains("egg-producing poultry")));
    }

    #[test]
    fn assesses_raw_potato_as_restricted_for_poultry() {
        let raw_potato = Feed {
            name_ru: "Из картофеля сырого".to_string(),
            category: "succulent".to_string(),
            ..Default::default()
        };

        let poultry_assessment =
            assess_feed_suitability(&raw_potato, "poultry", Some("poultry_broiler"));
        let cattle_assessment =
            assess_feed_suitability(&raw_potato, "cattle", Some("cattle_dairy"));

        assert_eq!(poultry_assessment.status, FeedSuitabilityStatus::Restricted);
        assert_eq!(cattle_assessment.status, FeedSuitabilityStatus::Conditional);
    }

    #[test]
    fn assesses_urea_as_restricted_for_swine_and_conditional_for_cattle() {
        let urea = Feed {
            name_ru: "Карбамид (мочевина)".to_string(),
            category: "nitrogen_compounds".to_string(),
            subcategory: Some("Карбамид".to_string()),
            ..Default::default()
        };

        let swine_assessment = assess_feed_suitability(&urea, "swine", Some("swine_finisher"));
        let cattle_assessment = assess_feed_suitability(&urea, "cattle", Some("cattle_beef_500"));

        assert_eq!(swine_assessment.status, FeedSuitabilityStatus::Restricted);
        assert!(swine_assessment
            .notes
            .iter()
            .any(|note| note.contains("not offered for swine or poultry")));
        assert_eq!(cattle_assessment.status, FeedSuitabilityStatus::Conditional);
        assert!(cattle_assessment
            .notes
            .iter()
            .any(|note| note.contains("acclimated ruminants")));
    }

    #[test]
    fn restricts_later_broiler_formula_for_starter_context() {
        let later_broiler_premix = Feed {
            name_ru: "Премикс для бройлеров старше 4 нед., молодняка птицы 4-8 нед.".to_string(),
            category: "premix".to_string(),
            ..Default::default()
        };

        let starter_assessment = assess_feed_suitability(
            &later_broiler_premix,
            "poultry",
            Some("poultry_broiler_starter"),
        );

        assert_eq!(starter_assessment.status, FeedSuitabilityStatus::Restricted);
        assert!(starter_assessment
            .notes
            .iter()
            .any(|note| note.contains("production phase")));
    }

    #[test]
    fn restricts_non_supported_species_formulas() {
        let goat_premix = Feed {
            name_ru: "Премикс для овец и коз".to_string(),
            category: "premix".to_string(),
            ..Default::default()
        };

        let cattle_assessment =
            assess_feed_suitability(&goat_premix, "cattle", Some("cattle_dairy_25"));

        assert_eq!(cattle_assessment.status, FeedSuitabilityStatus::Restricted);
        assert!(cattle_assessment
            .notes
            .iter()
            .any(|note| note.contains("outside the current cattle, swine, and poultry scope")));
    }
}
