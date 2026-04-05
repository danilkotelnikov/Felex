use anyhow::Result;
use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use url::Url;

const DEFAULT_REGION: &str = "Россия (среднее)";
const MARKET_DOMAINS: &[&str] = &[
    "agroserver.ru",
    "agrobazar.ru",
    "pulscen.ru",
    "agrolist.ru",
    "kormzerno.ru",
    "mcx.gov.ru",
    "zakupki.gov.ru",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchedPrice {
    pub feed_name: String,
    pub price_rubles_per_ton: f64,
    pub source: String,
    pub source_url: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Clone)]
struct SearchListing {
    url: String,
    title: String,
    snippet: String,
}

pub struct PriceFetcher {
    client: Client,
}

impl PriceFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(12))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Felex/2.0")
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    pub async fn fetch_all(&self) -> Result<Vec<FetchedPrice>> {
        let mut candidates = Vec::new();

        if let Ok(prices) = self.fetch_ikar().await {
            candidates.extend(prices);
        }
        if let Ok(prices) = self.fetch_mcx().await {
            candidates.extend(prices);
        }
        if let Ok(prices) = self.fetch_market_search().await {
            candidates.extend(prices);
        }

        let mut aggregated = Self::aggregate_prices(candidates);
        aggregated = self.backfill_missing(aggregated);

        if aggregated.is_empty() {
            Ok(self.fallback_prices())
        } else {
            Ok(aggregated)
        }
    }

    async fn fetch_ikar(&self) -> Result<Vec<FetchedPrice>> {
        let response = self.client.get("https://ikar.ru/lenta/").send().await?;
        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let body = response.text().await?;
        let document = Html::parse_document(&body);
        let selector = Selector::parse("table td, table.price-table td, .content td").unwrap();

        let mut prices = Vec::new();
        let mut current_feed: Option<String> = None;

        for element in document.select(&selector) {
            let text = element.text().collect::<String>().trim().to_string();
            if text.is_empty() {
                continue;
            }

            if let Some(feed_name) = Self::match_feed_name(&text) {
                current_feed = Some(feed_name);
                continue;
            }

            if let (Some(feed_name), Some(price)) =
                (current_feed.clone(), Self::parse_price_value(&text))
            {
                prices.push(FetchedPrice {
                    feed_name,
                    price_rubles_per_ton: price,
                    source: "ikar.ru".to_string(),
                    source_url: Some("https://ikar.ru/lenta/".to_string()),
                    region: Some(DEFAULT_REGION.to_string()),
                });
                current_feed = None;
            }
        }

        Ok(prices)
    }

    async fn fetch_mcx(&self) -> Result<Vec<FetchedPrice>> {
        let url = "https://mcx.gov.ru/ministry/departments/departament-regulirovaniya-prodovolstvennykh-rynkov-i-kachestva-produktsii/industry-information/info-monitoring/";
        let response = match self.client.get(url).send().await {
            Ok(response) if response.status().is_success() => response,
            _ => return Ok(Vec::new()),
        };

        let body = response.text().await?;
        let document = Html::parse_document(&body);
        let row_selector = Selector::parse("table tr").unwrap();
        let cell_selector = Selector::parse("td").unwrap();

        let mut prices = Vec::new();
        for row in document.select(&row_selector) {
            let cells: Vec<String> = row
                .select(&cell_selector)
                .map(|cell| cell.text().collect::<String>().trim().to_string())
                .filter(|cell| !cell.is_empty())
                .collect();

            if cells.len() < 2 {
                continue;
            }

            if let Some(feed_name) = Self::match_feed_name(&cells[0]) {
                if let Some(price) = cells
                    .iter()
                    .skip(1)
                    .find_map(|cell| Self::parse_price_value(cell))
                {
                    prices.push(FetchedPrice {
                        feed_name,
                        price_rubles_per_ton: price,
                        source: "mcx.gov.ru".to_string(),
                        source_url: Some(url.to_string()),
                        region: Some(DEFAULT_REGION.to_string()),
                    });
                }
            }
        }

        Ok(prices)
    }

    async fn fetch_market_search(&self) -> Result<Vec<FetchedPrice>> {
        let queries = [
            ("Пшеница фуражная", "пшеница фуражная цена"),
            ("Ячмень дробленый", "ячмень кормовой цена"),
            ("Кукуруза зерно", "кукуруза зерно цена"),
            ("Овес", "овес цена"),
            ("Шрот подсолнечный", "шрот подсолнечный цена"),
            ("Шрот соевый", "соевый шрот цена"),
            ("Шрот рапсовый", "рапсовый шрот цена"),
            ("Жом свекловичный сухой", "жом свекловичный сухой цена"),
            ("Отруби пшеничные", "отруби пшеничные цена"),
            ("Горох кормовой", "горох кормовой цена"),
            ("Сено люцерны", "сено люцерны цена"),
            ("Мел кормовой", "мел кормовой цена"),
            ("Соль поваренная", "соль кормовая цена"),
            ("Премикс П60-1 (для КРС)", "премикс крс цена"),
        ];

        let mut prices = Vec::new();
        for (feed_name, query) in queries {
            match self.fetch_search_results(feed_name, query).await {
                Ok(mut items) => prices.append(&mut items),
                Err(error) => tracing::debug!("market search skipped: {}", error),
            }
        }

        Ok(prices)
    }

    async fn fetch_search_results(
        &self,
        canonical_name: &str,
        query: &str,
    ) -> Result<Vec<FetchedPrice>> {
        let domain_scope = "(site:agroserver.ru OR site:agrobazar.ru OR site:pulscen.ru OR site:agrolist.ru OR site:kormzerno.ru OR site:mcx.gov.ru OR site:zakupki.gov.ru)";
        let final_query = format!("{} {} руб/кг OR руб/т", query, domain_scope);

        let response = self
            .client
            .get("https://html.duckduckgo.com/html/")
            .query(&[("q", final_query)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let body = response.text().await?;
        let listings = Self::parse_duckduckgo_results(&body);
        let mut prices = Vec::new();
        let target_key = Self::canonical_feed_key(canonical_name);

        let mut relevant_listings = listings
            .into_iter()
            .filter(|listing| Self::listing_matches_feed(target_key, listing))
            .collect::<Vec<_>>();

        relevant_listings.sort_by_key(|listing| Self::domain_priority(&listing.url));

        for listing in relevant_listings.into_iter().take(5) {
            let price = match Self::parse_price_with_unit(&listing.snippet) {
                Some(value) => value,
                None => continue,
            };

            let source =
                Self::domain_from_url(&listing.url).unwrap_or_else(|| "duckduckgo".to_string());
            prices.push(FetchedPrice {
                feed_name: canonical_name.to_string(),
                price_rubles_per_ton: price,
                source,
                source_url: Some(listing.url),
                region: Some(DEFAULT_REGION.to_string()),
            });
        }

        Ok(prices)
    }

    fn parse_duckduckgo_results(body: &str) -> Vec<SearchListing> {
        let document = Html::parse_document(body);
        let result_selector = Selector::parse("div.result").unwrap();
        let title_selector = Selector::parse("a.result__a").unwrap();
        let snippet_selector = Selector::parse(".result__snippet").unwrap();

        document
            .select(&result_selector)
            .filter_map(|result| {
                let link = result.select(&title_selector).next()?;
                let href = link.value().attr("href")?;
                let url = Self::resolve_duckduckgo_link(href);
                let title = link.text().collect::<String>().trim().to_string();
                let snippet = result
                    .select(&snippet_selector)
                    .next()
                    .map(|node| node.text().collect::<String>())
                    .unwrap_or_else(|| title.clone());

                Some(SearchListing {
                    url,
                    title,
                    snippet: snippet.trim().to_string(),
                })
            })
            .collect()
    }

    fn resolve_duckduckgo_link(raw: &str) -> String {
        if raw.starts_with("http") {
            return raw.to_string();
        }

        if let Ok(url) = Url::parse(&format!("https://duckduckgo.com{}", raw)) {
            if let Some(target) = url
                .query_pairs()
                .find(|(key, _)| key == "uddg")
                .map(|(_, value)| value.to_string())
            {
                return target;
            }
        }

        raw.to_string()
    }

    fn aggregate_prices(prices: Vec<FetchedPrice>) -> Vec<FetchedPrice> {
        let mut grouped: HashMap<String, Vec<FetchedPrice>> = HashMap::new();
        for price in prices {
            grouped
                .entry(price.feed_name.clone())
                .or_default()
                .push(price);
        }

        grouped
            .into_iter()
            .filter_map(|(feed_name, mut group)| {
                group.retain(|entry| {
                    entry.price_rubles_per_ton > 100.0 && entry.price_rubles_per_ton < 500_000.0
                });
                if group.is_empty() {
                    return None;
                }

                group.sort_by(|left, right| {
                    left.price_rubles_per_ton
                        .total_cmp(&right.price_rubles_per_ton)
                });
                let median = if group.len() % 2 == 1 {
                    group[group.len() / 2].price_rubles_per_ton
                } else {
                    let upper = group.len() / 2;
                    (group[upper - 1].price_rubles_per_ton + group[upper].price_rubles_per_ton)
                        / 2.0
                };

                let sources = group
                    .iter()
                    .map(|entry| entry.source.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                let median_index = group.len() / 2;
                let representative_url = group
                    .iter()
                    .filter_map(|entry| entry.source_url.as_ref())
                    .min_by_key(|url| Self::domain_priority(url))
                    .cloned()
                    .or_else(|| {
                        group
                            .get(median_index)
                            .and_then(|entry| entry.source_url.clone())
                    });

                Some(FetchedPrice {
                    feed_name,
                    price_rubles_per_ton: median,
                    source: if sources.len() == 1 {
                        sources[0].clone()
                    } else {
                        format!("aggregated:{}", sources.join("|"))
                    },
                    source_url: representative_url,
                    region: Some(DEFAULT_REGION.to_string()),
                })
            })
            .collect()
    }

    fn backfill_missing(&self, current: Vec<FetchedPrice>) -> Vec<FetchedPrice> {
        let mut present = current
            .iter()
            .filter_map(|price| Self::canonical_feed_key(&price.feed_name))
            .collect::<BTreeSet<_>>();
        let mut combined = current;

        for fallback in self.fallback_prices() {
            if let Some(key) = Self::canonical_feed_key(&fallback.feed_name) {
                if present.insert(key) {
                    combined.push(fallback);
                }
            }
        }

        combined
    }

    fn domain_from_url(raw: &str) -> Option<String> {
        Url::parse(raw).ok().and_then(|url| {
            url.domain()
                .map(|domain| domain.trim_start_matches("www.").to_string())
        })
    }

    fn domain_priority(raw: &str) -> usize {
        let Some(domain) = Self::domain_from_url(raw) else {
            return MARKET_DOMAINS.len() + 10;
        };

        MARKET_DOMAINS
            .iter()
            .position(|candidate| domain == *candidate)
            .unwrap_or(MARKET_DOMAINS.len() + 1)
    }

    fn feed_search_terms(key: Option<&str>) -> &'static [&'static str] {
        match key {
            Some("feed_wheat") => &["пшениц", "фуражн", "feed wheat", "wheat"],
            Some("barley") => &["ячмен", "barley"],
            Some("corn_grain") => &["кукуруз", "corn", "maize"],
            Some("oats") => &["овес", "oats"],
            Some("sunflower_meal") => &["подсолнеч", "sunflower meal"],
            Some("soybean_meal") => &["соев", "soybean meal", "soy meal"],
            Some("rapeseed_meal") => &["рапсов", "rapeseed meal", "canola meal"],
            Some("fish_meal") => &["рыбн", "fish meal"],
            Some("wheat_bran") => &["отруб", "wheat bran", "bran"],
            Some("beet_pulp") => &["жом", "свекл", "beet pulp"],
            Some("feed_peas") => &["горох", "peas", "feed peas"],
            Some("corn_silage") => &["силос", "кукуруз", "corn silage"],
            Some("alfalfa_hay") => &["люцерн", "alfalfa hay", "hay"],
            Some("feed_chalk") => &["мел", "chalk", "feed chalk"],
            Some("feed_salt") => &["соль", "salt", "feed salt"],
            Some("monocalcium_phosphate") => &["монокальц", "monocalcium phosphate", "mcp"],
            Some("dicalcium_phosphate") => &["дикальц", "dicalcium phosphate", "dcp"],
            Some("feed_limestone") => &["известня", "limestone", "calcium carbonate"],
            Some("layer_shell_grit") => &["ракуш", "shell grit", "shell meal", "несуш"],
            Some("cattle_premix") => &["премикс", "крс", "cattle premix"],
            Some("layer_premix") => &["премикс", "несуш", "layer premix"],
            Some("swine_premix") => &["премикс", "свин", "swine premix", "pig premix"],
            Some("broiler_premix") => &["премикс", "бройл", "broiler premix"],
            Some("poultry_bvmk") => &["бвмк", "птиц", "poultry bvmk"],
            _ => &[],
        }
    }

    fn listing_matches_feed(target_key: Option<&str>, listing: &SearchListing) -> bool {
        let Some(target_key) = target_key else {
            return true;
        };

        let combined = Self::normalize_text(&format!(
            "{} {} {}",
            listing.title, listing.snippet, listing.url
        ));

        if let Some(detected_key) = Self::canonical_feed_key(&combined) {
            if detected_key != target_key {
                return false;
            }
        }

        Self::feed_search_terms(Some(target_key))
            .iter()
            .any(|term| combined.contains(term))
    }

    fn normalize_text(text: &str) -> String {
        text.to_lowercase().replace('ё', "е").replace('\u{a0}', " ")
    }

    fn canonical_feed_key(text: &str) -> Option<&'static str> {
        let lower = Self::normalize_text(text);

        if lower.contains("пшениц")
            || lower.contains("feed wheat")
            || (lower.contains("wheat") && lower.contains("feed"))
        {
            return Some("feed_wheat");
        }
        if lower.contains("ячмен") || lower.contains("barley") {
            return Some("barley");
        }
        if lower.contains("кукуруз")
            || lower.contains("corn grain")
            || lower.contains("cracked corn")
        {
            return Some("corn_grain");
        }
        if lower.contains("овес") || lower.contains("oats") {
            return Some("oats");
        }
        if (lower.contains("подсолнеч") && lower.contains("шрот"))
            || lower.contains("sunflower meal")
        {
            return Some("sunflower_meal");
        }
        if (lower.contains("соев") && lower.contains("шрот")) || lower.contains("soybean meal")
        {
            return Some("soybean_meal");
        }
        if (lower.contains("рапсов") && lower.contains("шрот")) || lower.contains("rapeseed meal")
        {
            return Some("rapeseed_meal");
        }
        if (lower.contains("рыбн") && lower.contains("мук")) || lower.contains("fish meal") {
            return Some("fish_meal");
        }
        if lower.contains("отруб") || lower.contains("wheat bran") {
            return Some("wheat_bran");
        }
        if (lower.contains("жом") && lower.contains("свекл")) || lower.contains("beet pulp")
        {
            return Some("beet_pulp");
        }
        if lower.contains("горох") || lower.contains("feed peas") || lower.contains("peas") {
            return Some("feed_peas");
        }
        if (lower.contains("силос") && lower.contains("кукуруз")) || lower.contains("corn silage")
        {
            return Some("corn_silage");
        }
        if (lower.contains("сено") && lower.contains("люцерн")) || lower.contains("alfalfa hay")
        {
            return Some("alfalfa_hay");
        }
        if (lower.contains("мел") && lower.contains("корм")) || lower.contains("feed chalk")
        {
            return Some("feed_chalk");
        }
        if (lower.contains("соль") && (lower.contains("повар") || lower.contains("корм")))
            || lower.contains("feed salt")
        {
            return Some("feed_salt");
        }
        if (lower.contains("монокальц") && lower.contains("фосфат"))
            || lower.contains("monocalcium phosphate")
            || lower.contains("monocalcium_phosphate")
            || lower.contains("mcp")
        {
            return Some("monocalcium_phosphate");
        }
        if (lower.contains("дикальц") && lower.contains("фосфат"))
            || lower.contains("dicalcium phosphate")
            || lower.contains("dicalcium_phosphate")
            || lower.contains("dcp")
        {
            return Some("dicalcium_phosphate");
        }
        if lower.contains("известня")
            || lower.contains("limestone")
            || lower.contains("calcium carbonate")
        {
            return Some("feed_limestone");
        }
        if lower.contains("ракуш")
            || lower.contains("shell grit")
            || lower.contains("shell_grit")
            || lower.contains("shell meal")
        {
            return Some("layer_shell_grit");
        }
        if (lower.contains("премикс") || lower.contains("premix"))
            && (lower.contains("крс") || lower.contains("cattle"))
        {
            return Some("cattle_premix");
        }
        if (lower.contains("премикс") || lower.contains("premix"))
            && (lower.contains("несуш") || lower.contains("layer"))
        {
            return Some("layer_premix");
        }
        if (lower.contains("премикс") || lower.contains("premix"))
            && (lower.contains("свин") || lower.contains("swine") || lower.contains("pig"))
        {
            return Some("swine_premix");
        }
        if (lower.contains("премикс") || lower.contains("premix"))
            && (lower.contains("бройл") || lower.contains("broiler"))
        {
            return Some("broiler_premix");
        }
        if lower.contains("бвмк") && (lower.contains("птиц") || lower.contains("poultry")) {
            return Some("poultry_bvmk");
        }

        None
    }

    fn match_feed_name(text: &str) -> Option<String> {
        match Self::canonical_feed_key(text)? {
            "feed_wheat" => Some("Пшеница фуражная".to_string()),
            "barley" => Some("Ячмень дробленый".to_string()),
            "corn_grain" => Some("Кукуруза зерно".to_string()),
            "oats" => Some("Овес".to_string()),
            "sunflower_meal" => Some("Шрот подсолнечный".to_string()),
            "soybean_meal" => Some("Шрот соевый".to_string()),
            "rapeseed_meal" => Some("Шрот рапсовый".to_string()),
            "fish_meal" => Some("Рыбная мука".to_string()),
            "wheat_bran" => Some("Отруби пшеничные".to_string()),
            "beet_pulp" => Some("Жом свекловичный сухой".to_string()),
            "feed_peas" => Some("Горох кормовой".to_string()),
            "corn_silage" => Some("Силос кукурузный".to_string()),
            "alfalfa_hay" => Some("Сено люцерны".to_string()),
            "feed_chalk" => Some("Мел кормовой".to_string()),
            "feed_salt" => Some("Соль поваренная".to_string()),
            "monocalcium_phosphate" => Some("Монокальцийфосфат".to_string()),
            "dicalcium_phosphate" => Some("Дикальцийфосфат кормовой".to_string()),
            "feed_limestone" => Some("Известняковая мука кормовая".to_string()),
            "layer_shell_grit" => Some("Ракушка кормовая для несушек".to_string()),
            "cattle_premix" => Some("Премикс П60-1 (для КРС)".to_string()),
            "layer_premix" => Some("Премикс для кур-несушек ZooExpert".to_string()),
            "swine_premix" => Some("Премикс для свиней 1%".to_string()),
            "broiler_premix" => Some("Премикс для бройлеров 1%".to_string()),
            "poultry_bvmk" => {
                Some("БВМК с пробиотиком для сельхозптицы Добрый Селянин".to_string())
            }
            _ => None,
        }
    }

    fn parse_price_value(text: &str) -> Option<f64> {
        let cleaned: String = text
            .replace(',', ".")
            .replace('\u{a0}', "")
            .replace(' ', "")
            .chars()
            .filter(|ch| ch.is_ascii_digit() || *ch == '.')
            .collect();

        cleaned
            .parse::<f64>()
            .ok()
            .filter(|price| *price > 100.0 && *price < 500_000.0)
    }

    fn parse_price_with_unit(text: &str) -> Option<f64> {
        let normalized = Self::normalize_text(text);
        let regex = Regex::new(r"(?P<price>\d{1,3}(?:[ \u{a0}]\d{3})*(?:[\.,]\d+)?)\s*(?:руб(?:\.|лей|ля|ль)?|₽)\s*(?:/|за\s*)(?P<unit>кг|kg|т|тонн?[аы]?|ton)").ok()?;
        let captures = regex.captures(&normalized)?;
        let price_text = captures
            .name("price")?
            .as_str()
            .replace(' ', "")
            .replace('\u{a0}', "")
            .replace(',', ".");
        let unit = captures.name("unit")?.as_str();
        let mut value = price_text.parse::<f64>().ok()?;

        if unit == "кг" || unit == "kg" {
            value *= 1000.0;
        }

        if value > 100.0 && value < 500_000.0 {
            Some(value)
        } else {
            None
        }
    }

    pub fn fallback_prices(&self) -> Vec<FetchedPrice> {
        vec![
            FetchedPrice {
                feed_name: "Пшеница фуражная".to_string(),
                price_rubles_per_ton: 14500.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Ячмень дробленый".to_string(),
                price_rubles_per_ton: 12000.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Кукуруза зерно".to_string(),
                price_rubles_per_ton: 15000.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Овес".to_string(),
                price_rubles_per_ton: 9500.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Шрот подсолнечный".to_string(),
                price_rubles_per_ton: 22000.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Шрот соевый".to_string(),
                price_rubles_per_ton: 42000.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Шрот рапсовый".to_string(),
                price_rubles_per_ton: 25000.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Жом свекловичный сухой".to_string(),
                price_rubles_per_ton: 6000.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Отруби пшеничные".to_string(),
                price_rubles_per_ton: 7500.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Горох кормовой".to_string(),
                price_rubles_per_ton: 18000.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Силос кукурузный".to_string(),
                price_rubles_per_ton: 900.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Сено люцерны".to_string(),
                price_rubles_per_ton: 8000.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Мел кормовой".to_string(),
                price_rubles_per_ton: 3500.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Соль поваренная".to_string(),
                price_rubles_per_ton: 5000.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
            FetchedPrice {
                feed_name: "Премикс П60-1 (для КРС)".to_string(),
                price_rubles_per_ton: 85000.0,
                source: "fallback".to_string(),
                source_url: None,
                region: Some(DEFAULT_REGION.to_string()),
            },
        ]
    }
}

pub fn map_prices_to_feeds(
    fetched: &[FetchedPrice],
    feed_names: &HashMap<String, i64>,
) -> Vec<(i64, f64, String, Option<String>, Option<String>)> {
    fetched
        .iter()
        .filter_map(|price| {
            let price_key = PriceFetcher::canonical_feed_key(&price.feed_name);

            let feed_id = price_key
                .and_then(|key| {
                    feed_names.iter().find_map(|(name, id)| {
                        if PriceFetcher::canonical_feed_key(name) == Some(key) {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                })
                .or_else(|| {
                    feed_names.get(&price.feed_name).copied().or_else(|| {
                        let price_name = PriceFetcher::normalize_text(&price.feed_name);
                        feed_names.iter().find_map(|(name, id)| {
                            let db_name = PriceFetcher::normalize_text(name);
                            if db_name.contains(&price_name) || price_name.contains(&db_name) {
                                Some(*id)
                            } else {
                                None
                            }
                        })
                    })
                });

            feed_id.map(|id| {
                (
                    id,
                    price.price_rubles_per_ton,
                    price.source.clone(),
                    price.region.clone(),
                    price.source_url.clone(),
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::PriceFetcher;

    #[test]
    fn recognizes_new_seed_price_tracking_keys() {
        assert_eq!(
            PriceFetcher::canonical_feed_key("dicalcium_phosphate"),
            Some("dicalcium_phosphate")
        );
        assert_eq!(
            PriceFetcher::canonical_feed_key("layer_shell_grit"),
            Some("layer_shell_grit")
        );
        assert_eq!(
            PriceFetcher::canonical_feed_key("swine_premix"),
            Some("swine_premix")
        );
        assert_eq!(
            PriceFetcher::match_feed_name("broiler_premix"),
            Some("Премикс для бройлеров 1%".to_string())
        );
    }
}
