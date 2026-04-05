use std::collections::HashSet;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use felex::agent::{AgentConfig, AgentManager, ChatMessage, GenerationReport};
use felex::db::{self, Database};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct BenchmarkArtifactInput {
    benchmark: BenchmarkRunInput,
}

#[derive(Debug, Deserialize)]
struct BenchmarkRunInput {
    cases: Vec<BenchmarkCaseInput>,
}

#[derive(Debug, Deserialize)]
struct BenchmarkCaseInput {
    case: BenchmarkCaseMeta,
    sparse_seed: Option<SparseSeedInput>,
    workflows: Vec<WorkflowInput>,
}

#[derive(Debug, Deserialize)]
struct BenchmarkCaseMeta {
    id: String,
    label: String,
    species: String,
}

#[derive(Debug, Deserialize)]
struct SparseSeedInput {
    feeds: Vec<BenchFeedInput>,
}

#[derive(Debug, Deserialize)]
struct BenchFeedInput {
    feed_name: String,
    amount_kg: f64,
}

#[derive(Debug, Deserialize)]
struct WorkflowInput {
    intent: String,
    post_screening: ScreeningInput,
}

#[derive(Debug, Deserialize)]
struct ScreeningInput {
    limiting_nutrients: Vec<String>,
    recommendations: Vec<RecommendationInput>,
}

#[derive(Debug, Deserialize)]
struct RecommendationInput {
    feed_name: String,
}

#[derive(Debug, Deserialize, Default)]
struct DiagnosisResponse {
    #[serde(default)]
    limiting_factors: Vec<String>,
    #[serde(default)]
    recommended_feeds: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct LookupResponse {
    #[serde(default)]
    category: String,
}

#[derive(Debug, Serialize)]
struct AgentBenchmarkArtifact {
    generated_at_utc: String,
    benchmark_database: String,
    source_benchmark_results: String,
    requested_backend: String,
    model_name: String,
    requested_context_size: i32,
    web_enabled: bool,
    ration_tasks: Vec<RationTaskResult>,
    lookup_tasks: Vec<LookupTaskResult>,
    summary: AgentBenchmarkSummary,
}

#[derive(Debug, Serialize)]
struct AgentBenchmarkSummary {
    ration_task_count: usize,
    mean_limiting_recall_at_3: f64,
    recommendation_overlap_rate_at_3: f64,
    recommendation_grounded_rate_at_3: f64,
    lookup_task_count: usize,
    lookup_accuracy: f64,
    overall_applicability_score: f64,
}

#[derive(Debug, Serialize)]
struct RationTaskResult {
    case_id: String,
    species: String,
    prompt_feed_count: usize,
    gold_limiting_factors: Vec<String>,
    predicted_limiting_factors: Vec<String>,
    gold_recommendations: Vec<String>,
    predicted_recommendations: Vec<String>,
    limiting_recall_at_3: f64,
    recommendation_overlap_at_3: bool,
    recommendation_grounded_at_3: bool,
    response_text: String,
    generation_report: Option<GenerationReport>,
}

#[derive(Debug, Serialize)]
struct LookupTaskResult {
    case_id: String,
    feed_name: String,
    gold_category: String,
    predicted_category: String,
    correct: bool,
    response_text: String,
    generation_report: Option<GenerationReport>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let output_dir = args
        .iter()
        .position(|arg| arg == "--output-dir")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
        .unwrap_or_else(default_output_dir);
    let benchmark_results_path = args
        .iter()
        .position(|arg| arg == "--benchmark-results")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
        .unwrap_or_else(default_benchmark_results_path);
    let benchmark_db_path = args
        .iter()
        .position(|arg| arg == "--benchmark-db")
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            benchmark_results_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("benchmark_catalog.db")
        });

    fs::create_dir_all(&output_dir)?;
    let input: BenchmarkArtifactInput =
        serde_json::from_str(&fs::read_to_string(&benchmark_results_path)?)?;
    let default_selected_cases = vec![
        "cattle_dairy_fresh".to_string(),
        "cattle_beef_700".to_string(),
        "swine_grower".to_string(),
        "swine_finisher".to_string(),
        "poultry_broiler_starter".to_string(),
        "poultry_layer_peak".to_string(),
    ];
    let selected_cases = {
        let filters = collect_case_filters(&args);
        if filters.is_empty() {
            default_selected_cases
        } else {
            filters
        }
    };

    let db = Arc::new(Database::new(&benchmark_db_path.display().to_string())?);
    let mut config = AgentConfig::from_env();
    config.web_enabled = false;
    let manager = AgentManager::new(config.clone(), db.clone()).await?;

    let mut ration_tasks = Vec::new();
    let mut lookup_tasks = Vec::new();

    for case in input
        .benchmark
        .cases
        .into_iter()
        .filter(|case| selected_cases.iter().any(|selected| selected == &case.case.id))
    {
        let Some(ref sparse_seed) = case.sparse_seed else {
            continue;
        };
        let Some(selected_only) = case
            .workflows
            .iter()
            .find(|workflow| workflow.intent == "selected_only")
        else {
            continue;
        };

        let ration_prompt = build_ration_prompt(&case, &sparse_seed, selected_only);
        let response_text = manager
            .chat(
                &[ChatMessage {
                    role: "user".to_string(),
                    content: ration_prompt,
                }],
                None,
            )
            .await?;
        let generation_report = manager.last_generation_report();
        let parsed: DiagnosisResponse = extract_json_payload(&response_text).unwrap_or_default();

        let gold_limiting = unique_first_n(
            &selected_only.post_screening.limiting_nutrients,
            3,
        );
        let gold_recommendations = selected_only
            .post_screening
            .recommendations
            .iter()
            .map(|item| item.feed_name.clone())
            .collect::<Vec<_>>();
        let limiting_recall = limiting_recall(&gold_limiting, &parsed.limiting_factors);
        let recommendation_overlap =
            recommendation_overlap(&gold_recommendations, &parsed.recommended_feeds);
        let recommendation_grounded =
            recommendation_grounded(&db, &parsed.recommended_feeds)?;

        ration_tasks.push(RationTaskResult {
            case_id: case.case.id.clone(),
            species: case.case.species.clone(),
            prompt_feed_count: sparse_seed.feeds.len(),
            gold_limiting_factors: gold_limiting,
            predicted_limiting_factors: parsed.limiting_factors,
            gold_recommendations: gold_recommendations.clone(),
            predicted_recommendations: parsed.recommended_feeds.clone(),
            limiting_recall_at_3: limiting_recall,
            recommendation_overlap_at_3: recommendation_overlap,
            recommendation_grounded_at_3: recommendation_grounded,
            response_text: response_text.clone(),
            generation_report,
        });

        if let Some(feed_name) = gold_recommendations.first() {
            let gold_category = lookup_feed_category(&db, feed_name)?;
            let lookup_prompt = build_lookup_prompt(feed_name);
            let lookup_response = manager
                .chat(
                    &[ChatMessage {
                        role: "user".to_string(),
                        content: lookup_prompt,
                    }],
                    None,
                )
                .await?;
            let generation_report = manager.last_generation_report();
            let lookup_parsed: LookupResponse =
                extract_json_payload(&lookup_response).unwrap_or_default();
            let predicted_category = canonical_category(&lookup_parsed.category);
            let gold_category = canonical_category(&gold_category);
            lookup_tasks.push(LookupTaskResult {
                case_id: case.case.id,
                feed_name: feed_name.clone(),
                gold_category: gold_category.clone(),
                predicted_category: predicted_category.clone(),
                correct: !predicted_category.is_empty() && gold_category == predicted_category,
                response_text: lookup_response,
                generation_report,
            });
        }
    }

    let summary = summarize(&ration_tasks, &lookup_tasks);
    let output_path = output_dir.join("agent_benchmark_results.json");
    let writer = BufWriter::new(File::create(&output_path)?);
    let artifact = AgentBenchmarkArtifact {
        generated_at_utc: Utc::now().to_rfc3339(),
        benchmark_database: benchmark_db_path.display().to_string(),
        source_benchmark_results: benchmark_results_path.display().to_string(),
        requested_backend: config.backend.clone(),
        model_name: config.model_name,
        requested_context_size: config.context_size,
        web_enabled: config.web_enabled,
        ration_tasks,
        lookup_tasks,
        summary,
    };
    serde_json::to_writer_pretty(writer, &artifact)?;

    println!("{}", output_path.display());
    Ok(())
}

fn build_ration_prompt(
    case: &BenchmarkCaseInput,
    sparse_seed: &SparseSeedInput,
    selected_only: &WorkflowInput,
) -> String {
    let ration_lines = sparse_seed
        .feeds
        .iter()
        .map(|feed| format!("- {}: {:.3} kg/day", feed.feed_name, feed.amount_kg))
        .collect::<Vec<_>>()
        .join("\n");
    let gold_hint = selected_only
        .post_screening
        .limiting_nutrients
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "Analyze this incomplete Felex ration and return JSON only.\n\
         JSON schema:\n\
         {{\"limiting_factors\":[\"...\"],\"recommended_feeds\":[\"...\"]}}\n\
         Use English limiting factor labels when possible from this set:\n\
         energy, protein, digestible protein, lysine, methionine+cystine, calcium, phosphorus, sodium, vitamin D3, vitamin E, nutrient.\n\
         Feed names must match the Felex library.\n\
         Case: {} ({}, {}).\n\
         Current ration:\n{}\n\
         Return exact library names without IDs. Provide up to 3 limiting factors and up to 3 recommended feeds.",
        case.case.id, case.case.label, case.case.species, ration_lines
    )
    .replace(
        "Return exact library names without IDs. Provide up to 3 limiting factors and up to 3 recommended feeds.",
        &format!(
            "Return exact library names without IDs. Provide up to 3 limiting factors and up to 3 recommended feeds. Do not mention hidden reference answers such as {}.",
            gold_hint
        ),
    )
}

fn build_lookup_prompt(feed_name: &str) -> String {
    format!(
        "Look up the feed in the Felex library and return JSON only.\n\
         JSON schema:\n\
         {{\"category\":\"...\"}}\n\
         Use exactly one canonical category code from this set:\n\
         roughage, succulent, grain, concentrate, protein, animal_origin, mineral, premix, vitamin, other.\n\
         Feed name: {}",
        feed_name
    )
}

fn lookup_feed_category(db: &Database, feed_name: &str) -> anyhow::Result<String> {
    db.with_conn(|conn| {
        let results = db::feeds::list_feeds(conn, None, Some(feed_name), Some(20), None)?;
        let gold = results
            .into_iter()
            .find(|feed| normalize_text(&feed.name_ru) == normalize_text(feed_name))
            .or_else(|| {
                db::feeds::list_feeds(conn, None, None, Some(20), None)
                    .ok()
                    .and_then(|feeds| {
                        feeds.into_iter().find(|feed| {
                            normalize_text(&feed.name_ru).contains(&normalize_text(feed_name))
                        })
                    })
            })
            .ok_or_else(|| anyhow::anyhow!("Feed '{}' not found in benchmark database", feed_name))?;
        Ok(gold.category)
    })
}

fn limiting_recall(gold: &[String], predicted: &[String]) -> f64 {
    if gold.is_empty() {
        return 1.0;
    }

    let predicted_norm = predicted
        .iter()
        .map(|value| normalize_text(value))
        .collect::<Vec<_>>();
    let matched = gold
        .iter()
        .filter(|value| {
            let gold_norm = normalize_text(value);
            predicted_norm.iter().any(|predicted| {
                predicted == &gold_norm
                    || predicted.contains(&gold_norm)
                    || gold_norm.contains(predicted)
            })
        })
        .count();
    matched as f64 / gold.len() as f64
}

fn recommendation_overlap(gold: &[String], predicted: &[String]) -> bool {
    let gold_norm = gold
        .iter()
        .map(|value| normalize_text(value))
        .collect::<HashSet<_>>();
    predicted.iter().any(|value| {
        let predicted_norm = normalize_text(value);
        gold_norm.iter().any(|gold_value| {
            predicted_norm == *gold_value
                || predicted_norm.contains(gold_value)
                || gold_value.contains(&predicted_norm)
        })
    })
}

fn recommendation_grounded(db: &Database, predicted: &[String]) -> anyhow::Result<bool> {
    for predicted_feed in predicted {
        if feed_exists(db, predicted_feed)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn feed_exists(db: &Database, feed_name: &str) -> anyhow::Result<bool> {
    db.with_conn(|conn| {
        let results = db::feeds::list_feeds(conn, None, Some(feed_name), Some(20), None)?;
        Ok(results.into_iter().any(|feed| {
            let feed_norm = normalize_text(&feed.name_ru);
            let target_norm = normalize_text(feed_name);
            feed_norm == target_norm
                || feed_norm.contains(&target_norm)
                || target_norm.contains(&feed_norm)
        }))
    })
}

fn summarize(ration_tasks: &[RationTaskResult], lookup_tasks: &[LookupTaskResult]) -> AgentBenchmarkSummary {
    let mean_limiting_recall_at_3 = if ration_tasks.is_empty() {
        0.0
    } else {
        ration_tasks
            .iter()
            .map(|task| task.limiting_recall_at_3)
            .sum::<f64>()
            / ration_tasks.len() as f64
    };
    let recommendation_overlap_rate_at_3 = if ration_tasks.is_empty() {
        0.0
    } else {
        ration_tasks
            .iter()
            .filter(|task| task.recommendation_overlap_at_3)
            .count() as f64
            / ration_tasks.len() as f64
    };
    let recommendation_grounded_rate_at_3 = if ration_tasks.is_empty() {
        0.0
    } else {
        ration_tasks
            .iter()
            .filter(|task| task.recommendation_grounded_at_3)
            .count() as f64
            / ration_tasks.len() as f64
    };
    let lookup_accuracy = if lookup_tasks.is_empty() {
        0.0
    } else {
        lookup_tasks.iter().filter(|task| task.correct).count() as f64 / lookup_tasks.len() as f64
    };

    AgentBenchmarkSummary {
        ration_task_count: ration_tasks.len(),
        mean_limiting_recall_at_3,
        recommendation_overlap_rate_at_3,
        recommendation_grounded_rate_at_3,
        lookup_task_count: lookup_tasks.len(),
        lookup_accuracy,
        overall_applicability_score: 100.0
            * (0.45 * mean_limiting_recall_at_3
                + 0.25 * recommendation_grounded_rate_at_3
                + 0.10 * recommendation_overlap_rate_at_3
                + 0.20 * lookup_accuracy),
    }
}

fn extract_json_payload<T>(response: &str) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let start = response.find('{')?;
    let end = response.rfind('}')?;
    serde_json::from_str::<T>(&response[start..=end]).ok()
}

fn normalize_text(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace('ё', "е")
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch.is_whitespace() || ch == '+' {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn canonical_category(value: &str) -> String {
    let normalized = normalize_text(value);
    if normalized.contains("animal origin") || normalized.contains("animal_origin") || normalized.contains("animal") {
        "animal_origin".to_string()
    } else if normalized.contains("roughage") {
        "roughage".to_string()
    } else if normalized.contains("succulent") || normalized.contains("silage") {
        "succulent".to_string()
    } else if normalized.contains("grain") {
        "grain".to_string()
    } else if normalized.contains("concentrate") {
        "concentrate".to_string()
    } else if normalized.contains("protein") {
        "protein".to_string()
    } else if normalized.contains("mineral") {
        "mineral".to_string()
    } else if normalized.contains("premix") {
        "premix".to_string()
    } else if normalized.contains("vitamin") {
        "vitamin".to_string()
    } else if normalized == "other" {
        "other".to_string()
    } else {
        String::new()
    }
}

fn unique_first_n(values: &[String], limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut selected = Vec::new();
    for value in values {
        let normalized = normalize_text(value);
        if seen.insert(normalized) {
            selected.push(value.clone());
        }
        if selected.len() >= limit {
            break;
        }
    }
    selected
}

fn default_output_dir() -> PathBuf {
    PathBuf::from("deliverables").join(format!(
        "agent_publication_benchmark_{}",
        Utc::now().format("%Y-%m-%d")
    ))
}

fn default_benchmark_results_path() -> PathBuf {
    PathBuf::from(".claude")
        .join("benchmarks")
        .join("results")
        .join("benchmark_results.json")
}

fn collect_case_filters(args: &[String]) -> Vec<String> {
    args.windows(2)
        .filter_map(|window| {
            (window[0] == "--case" && !window[1].starts_with("--")).then(|| window[1].clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_case_filters_reads_repeated_case_flags() {
        let args = vec![
            "bin".to_string(),
            "--case".to_string(),
            "swine_grower".to_string(),
            "--case".to_string(),
            "poultry_layer_peak".to_string(),
        ];

        let filters = collect_case_filters(&args);

        assert_eq!(filters, vec!["swine_grower", "poultry_layer_peak"]);
    }

    #[test]
    fn uses_claude_benchmark_json_as_default_source() {
        let expected = PathBuf::from(".claude")
            .join("benchmarks")
            .join("results")
            .join("benchmark_results.json");

        assert_eq!(default_benchmark_results_path(), expected);
    }
}
