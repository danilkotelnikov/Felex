use std::collections::HashSet;

fn normalized_chars(value: &str) -> Vec<char> {
    value.to_lowercase().chars().collect()
}

/// Calculate trigram similarity between two strings.
pub fn trigram_similarity(a: &str, b: &str) -> f64 {
    let a_chars = normalized_chars(a);
    let b_chars = normalized_chars(b);

    if a_chars.is_empty() || b_chars.is_empty() {
        return 0.0;
    }

    if a_chars.len() < 3 || b_chars.len() < 3 {
        let a_lower: String = a_chars.iter().collect();
        let b_lower: String = b_chars.iter().collect();
        if a_lower.contains(&b_lower) || b_lower.contains(&a_lower) {
            return 0.8;
        }
        return 0.0;
    }

    let a_trigrams: HashSet<(char, char, char)> = a_chars
        .windows(3)
        .map(|window| (window[0], window[1], window[2]))
        .collect();
    let b_trigrams: HashSet<(char, char, char)> = b_chars
        .windows(3)
        .map(|window| (window[0], window[1], window[2]))
        .collect();

    let intersection = a_trigrams.intersection(&b_trigrams).count();
    let total = a_trigrams.len() + b_trigrams.len();

    if total == 0 {
        0.0
    } else {
        (2 * intersection) as f64 / total as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_scores_near_one() {
        assert!(trigram_similarity("пшеница", "пшеница") > 0.99);
    }

    #[test]
    fn typo_tolerance_keeps_similarity_positive() {
        assert!(trigram_similarity("пшенца", "пшеница") > 0.4);
    }

    #[test]
    fn partial_match_scores_above_threshold() {
        assert!(trigram_similarity("пшен", "пшеница") > 0.3);
    }
}
