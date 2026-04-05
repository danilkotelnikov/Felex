//! System prompts for the feed advisor agent

/// Main system prompt for feed advisor
pub const SYSTEM_PROMPT: &str = r#"You are Felex, an expert AI assistant for animal feed ration formulation. You help farmers and nutritionists optimize feed rations for dairy cattle, beef cattle, swine, and poultry.

## Your Expertise

1. **Nutrient Requirements**: You understand species-specific nutritional needs including energy (EKE, OE), protein (CP, digestible protein, amino acids), fiber (crude fiber), minerals (Ca, P, trace minerals), and vitamins.

2. **Feed Ingredients**: You know the nutritional composition of common feeds including concentrates (grains, oilseed meals), roughages (hay, straw), silages, minerals, and premixes.

3. **Ration Balancing**: You can help optimize rations for:
   - Minimum cost while meeting nutritional requirements
   - Maximum production (milk, meat, eggs)
   - Specific health goals (rumen health, metabolic balance)

4. **Practical Considerations**: You consider feed availability, palatability, mixing practicality, and on-farm constraints.

## Guidelines

- Always provide specific, actionable recommendations.
- When suggesting feeds, include approximate amounts in kg/day.
- Explain the nutritional rationale for your suggestions.
- Consider cost-effectiveness when making recommendations.
- Alert users to potential issues (acidosis risk, mineral imbalances, etc.).
- Use metric units (kg, g, mg, MJ).
- Reference Russian feed standards when applicable.
- Respond in clean GitHub-flavored Markdown.
- If web search results are provided, synthesize them into a final answer instead of repeating raw snippets.
- When using web information, cite sources inline as [1], [2], etc. and finish with a `Sources:` list containing titles and URLs.
- Never stop after emitting a tool call. After a tool result is available, continue to a plain-text final answer.
- Treat the local Felex feed library as the authoritative source for feed names, IDs, and nutrient composition before guessing.

## Response Format

Keep responses concise but informative. Use:
- Bullet points for lists
- Tables for comparing options
- Bold for key values and recommendations

## Available Information

You may have access to:
- Feed database with nutritional values and prices
- Local feed library data resolved from the Felex SQLite database
- Nutrient requirements by animal type and production level
- Current ration composition and nutrient balance
- Web search for latest research and prices

When you need fresh external information, use the appropriate tools or any web results already provided in the conversation.
"#;

/// Create context-aware prompt with ration details
pub fn create_context_prompt(
    animal_type: &str,
    production_level: &str,
    current_ration: Option<&str>,
    nutrient_status: Option<&str>,
) -> String {
    let mut context = String::from(SYSTEM_PROMPT);

    context.push_str("\n\n## Current Context\n\n");
    context.push_str(&format!("**Animal Type**: {}\n", animal_type));
    context.push_str(&format!("**Production Level**: {}\n", production_level));

    if let Some(ration) = current_ration {
        context.push_str(&format!("\n**Current Ration**:\n{}\n", ration));
    }

    if let Some(status) = nutrient_status {
        context.push_str(&format!("\n**Nutrient Status**:\n{}\n", status));
    }

    context
}

/// Build ration context string from current ration data
pub fn build_ration_context(
    animal_type: &str,
    animal_properties: &serde_json::Value,
    feeds: &[(String, f64)], // (feed_name, amount_kg)
    nutrient_summary: &serde_json::Value,
    deficiencies: &[(String, f64, f64)], // (nutrient, actual, target)
) -> String {
    let mut lines = Vec::new();

    // Animal description
    let breed = animal_properties
        .get("breed")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let weight = animal_properties
        .get("liveWeightKg")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let milk = animal_properties
        .get("milkYieldKg")
        .and_then(|v| v.as_f64());
    let gain = animal_properties.get("dailyGainG").and_then(|v| v.as_f64());

    let mut animal_desc = format!(
        "Current ration for {} ({}, {:.0} kg",
        animal_type, breed, weight
    );
    if let Some(m) = milk {
        animal_desc.push_str(&format!(", {:.0} kg milk/day", m));
    }
    if let Some(g) = gain {
        animal_desc.push_str(&format!(", {:.0} g daily gain", g));
    }
    animal_desc.push_str("):");
    lines.push(animal_desc);

    // Feeds list
    if !feeds.is_empty() {
        let feed_parts: Vec<String> = feeds
            .iter()
            .map(|(name, kg)| format!("{} {:.1} kg", name, kg))
            .collect();
        lines.push(format!("Feeds: {}", feed_parts.join(", ")));
    }

    // Nutrient summary
    let mut nutrient_parts = Vec::new();
    if let Some(cp) = nutrient_summary
        .get("crude_protein")
        .and_then(|v| v.as_f64())
    {
        nutrient_parts.push(format!("CP {:.0}g", cp));
    }
    if let Some(oe) = nutrient_summary
        .get("energy_oe_cattle")
        .and_then(|v| v.as_f64())
    {
        nutrient_parts.push(format!("Energy {:.1} MJ OE", oe));
    }
    if let Some(eke) = nutrient_summary.get("energy_eke").and_then(|v| v.as_f64()) {
        nutrient_parts.push(format!("EKE {:.1}", eke));
    }
    if let Some(dm) = nutrient_summary.get("total_dm_kg").and_then(|v| v.as_f64()) {
        nutrient_parts.push(format!("DM {:.1} kg", dm));
    }
    if let Some(fiber) = nutrient_summary.get("crude_fiber").and_then(|v| v.as_f64()) {
        nutrient_parts.push(format!("Crude fiber {:.0}g", fiber));
    }
    if let Some(ca) = nutrient_summary.get("calcium").and_then(|v| v.as_f64()) {
        nutrient_parts.push(format!("Ca {:.1}g", ca));
    }
    if let Some(p) = nutrient_summary.get("phosphorus").and_then(|v| v.as_f64()) {
        nutrient_parts.push(format!("P {:.1}g", p));
    }
    if let Some(lys) = nutrient_summary.get("lysine").and_then(|v| v.as_f64()) {
        nutrient_parts.push(format!("Lys {:.1}g", lys));
    }
    if let Some(metcys) = nutrient_summary
        .get("methionine_cystine")
        .and_then(|v| v.as_f64())
    {
        nutrient_parts.push(format!("Met+Cys {:.1}g", metcys));
    }
    if !nutrient_parts.is_empty() {
        lines.push(format!("Nutrients: {}", nutrient_parts.join(", ")));
    }

    // Deficiencies
    if !deficiencies.is_empty() {
        let def_parts: Vec<String> = deficiencies
            .iter()
            .map(|(nutrient, actual, target)| {
                let diff = actual - target;
                let pct = if *target > 0.0 {
                    (actual / target * 100.0) as i64
                } else {
                    0
                };
                format!(
                    "{} {:.0}g ({:.0}% of target {:.0}g)",
                    nutrient, diff, pct, target
                )
            })
            .collect();
        lines.push(format!("Deficiencies: {}", def_parts.join(", ")));
    }

    lines.join("\n")
}

/// Tool use instructions
pub fn tool_instructions(web_enabled: bool) -> String {
    let web_tool = if web_enabled {
        "1. **web_search**: Search the web for feed prices, research, or nutritional information\n   - Use when: The user asks about current prices, latest research, regulations, or regional information\n   - Parameters: {\"query\": \"search terms\"}\n"
    } else {
        "1. **web_search**: Disabled in settings for this chat session\n   - Do not call this tool unless the user enables web search again.\n"
    };

    format!(
        r#"
You have access to the following tools:

{}
2. **feed_lookup**: Query the local Felex feed library stored in SQLite
   - Use when: The user asks about a specific feed, category, or nutrient-rich ingredients
   - Parameters: {{"feed_name": "name of feed"}}
   - Category search: {{"category": "concentrate"}}
   - Nutrient filter: {{"nutrient": "crude_protein", "min_value": 300}}

3. **calculate_nutrients**: Calculate total nutrients from a feed combination resolved against the local feed library
   - Use when: The user provides specific feeds and amounts
   - Parameters: {{"feeds": [{{"name": "feed", "amount_kg": 5.0}}]}}

4. **suggest_feed**: Find feeds in the local feed library that are rich in a specific nutrient
   - Use when: The ration has a nutrient deficiency and you need candidate feeds to fix it
   - Parameters: {{"nutrient": "crude_protein", "deficiency_amount": 350}}
   - Returns top feeds ranked by that nutrient content per kg

To use a tool, respond with:
<tool>tool_name</tool>
<params>{{"key": "value"}}</params>

After the tool result is returned, write a final answer in Markdown.
Do not leave the response as only a tool call.
If web results are already provided in the prompt, do not call `web_search` again unless they are clearly insufficient.
When the current ration context is provided, use it to give specific, targeted advice. Reference the actual feeds and nutrient values in your response.
When suggesting feeds from the database, include the feed ID so the user can add it to the ration. Format as: Feed: FeedName (ID: 123)
"#,
        web_tool
    )
}

/// Parse tool calls from LLM response
pub fn parse_tool_call(response: &str) -> Option<(String, serde_json::Value)> {
    let decoded = response
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");

    let combined_regex = regex::Regex::new(
        r"(?is)<tool>\s*([a-zA-Z0-9_-]+)\s*</tool>[\s\S]*?<params>\s*([\s\S]*?)\s*</params>",
    )
    .ok()?;

    let captures = combined_regex.captures(&decoded)?;
    let tool_name = captures.get(1)?.as_str().trim().to_string();
    let params_raw = captures.get(2)?.as_str().trim();

    let params_clean = params_raw
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let params: serde_json::Value = serde_json::from_str(params_clean).ok()?;
    Some((tool_name, params))
}

/// Format tool result for context
pub fn format_tool_result(tool_name: &str, result: &str) -> String {
    format!(
        "\n<tool_result name=\"{}\">\n{}\n</tool_result>\n\nNow provide the final answer in Markdown. If the result came from web search, summarize it and include a Sources list with URLs.\n",
        tool_name,
        result
    )
}
