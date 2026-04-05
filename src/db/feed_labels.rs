//! Feed label utilities: localized category names and display helpers.

/// Returns a localized display label for a feed category slug.
///
/// # Arguments
/// * `category` - the internal category slug (e.g. `"grain"`, `"silage"`)
/// * `locale`   - BCP-47 locale prefix, e.g. `"ru"` or `"en"`
///
/// Falls back to the English label when the locale is unrecognised, and to
/// the raw slug when the category itself is unknown.
pub fn get_category_label(category: &str, locale: &str) -> &'static str {
    match locale {
        "ru" => match category {
            "grain" => "Зерновые",
            "concentrate" => "Концентраты",
            "oilseed_meal" => "Жмыхи и шроты",
            "protein" => "Белковые корма",
            "roughage" => "Грубые корма",
            "silage" => "Силос",
            "succulent" => "Сочные корма",
            "green_forage" => "Зелёные корма",
            "animal_origin" => "Корма животного происхождения",
            "mineral" => "Минеральные добавки",
            "premix" => "Премиксы",
            "oil_fat" => "Масла и жиры",
            "byproduct" => "Побочные продукты",
            "compound_feed" => "Комбикорма",
            "additive" => "Кормовые добавки",
            "root_crops" => "Корнеплоды",
            "other" => "Прочие",
            _ => category_label_en(category),
        },
        _ => category_label_en(category),
    }
}

fn category_label_en(category: &str) -> &'static str {
    match category {
        "grain" => "Grains",
        "concentrate" => "Concentrates",
        "oilseed_meal" => "Oilseed meals",
        "protein" => "Protein feeds",
        "roughage" => "Roughage",
        "silage" => "Silage",
        "succulent" => "Succulent feeds",
        "green_forage" => "Green forage",
        "animal_origin" => "Animal-origin feeds",
        "mineral" => "Mineral supplements",
        "premix" => "Premixes",
        "oil_fat" => "Oils & fats",
        "byproduct" => "Byproducts",
        "compound_feed" => "Compound feeds",
        "additive" => "Feed additives",
        "root_crops" => "Root crops",
        "other" => "Other",
        _ => "Other",
    }
}

/// Returns a bilingual display name for a feed (Russian / English).
pub fn display_feed_name(feed: &super::feeds::Feed) -> String {
    if let Some(name_en) = &feed.name_en {
        if !name_en.trim().is_empty() {
            return format!("{} / {}", feed.name_ru, name_en.trim());
        }
    }
    feed.name_ru.clone()
}

/// Returns a display label for a feed's category.
pub fn display_feed_category(category: &str) -> String {
    get_category_label(category, "en").to_string()
}

/// Returns a classification string for a feed (category + subcategory).
pub fn display_feed_classification(feed: &super::feeds::Feed) -> String {
    let cat = get_category_label(&feed.category, "en");
    if let Some(sub) = &feed.subcategory {
        if !sub.trim().is_empty() {
            return format!("{} / {}", cat, sub.trim());
        }
    }
    cat.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_russian_labels() {
        assert_eq!(get_category_label("grain", "ru"), "Зерновые");
        assert_eq!(get_category_label("silage", "ru"), "Силос");
        assert_eq!(get_category_label("other", "ru"), "Прочие");
    }

    #[test]
    fn test_english_fallback() {
        assert_eq!(get_category_label("grain", "en"), "Grains");
        assert_eq!(get_category_label("roughage", "en"), "Roughage");
    }

    #[test]
    fn test_unknown_locale_falls_back_to_english() {
        assert_eq!(get_category_label("grain", "de"), "Grains");
    }

    #[test]
    fn test_unknown_category_returns_other() {
        assert_eq!(get_category_label("unknown_cat", "ru"), "Other");
        assert_eq!(get_category_label("unknown_cat", "en"), "Other");
    }
}
