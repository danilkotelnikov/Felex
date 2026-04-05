use anyhow::{anyhow, Context, Result};
use printpdf::{IndirectFontRef, Mm, PdfDocument, PdfDocumentReference, PdfLayerIndex, PdfPageIndex};
use rust_xlsxwriter::{Color, Format, FormatAlign, Workbook};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCommandRequest {
    pub format: String,
    pub report: ExportReport,
    #[serde(default)]
    pub options: ExportOptions,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOptions {
    pub destination_dir: Option<String>,
    pub file_name: Option<String>,
    pub font_family: Option<String>,
    pub appearance: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResponse {
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub title: String,
    pub subtitle: Option<String>,
    pub notes: String,
    pub generated_at: String,
    pub include_nutrients: bool,
    pub include_economics: bool,
    pub include_norms_comparison: bool,
    pub include_feed_details: bool,
    pub animal_count: i32,
    pub items: Vec<ExportFeedRow>,
    pub nutrient_rows: Vec<ExportNutrientRow>,
    pub total_cost_per_head: f64,
    pub total_cost: f64,
    pub total_kg_per_head: f64,
    pub total_kg: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFeedRow {
    pub name: String,
    pub category: String,
    pub amount_kg_per_head: f64,
    pub amount_kg_total: f64,
    pub dry_matter_pct: Option<f64>,
    pub price_per_ton: Option<f64>,
    pub cost_per_day_per_head: f64,
    pub cost_per_day_total: f64,
    pub is_locked: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportNutrientRow {
    pub label: String,
    pub actual: f64,
    pub unit: String,
    pub norm: Option<ExportNormRange>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportNormRange {
    pub min: Option<f64>,
    pub target: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Clone, Copy)]
struct AppearanceProfile {
    title_size: f32,
    section_size: f32,
    body_size: f32,
    line_height_mm: f32,
    wrapped_line_height_mm: f32,
    max_chars: usize,
    xlsx_row_height: f64,
    xlsx_column_scale: f64,
    xlsx_heading_bg: &'static str,
    xlsx_heading_fg: &'static str,
}

#[derive(Clone, Copy)]
struct ExportFontSpec {
    xlsx_name: &'static str,
    pdf_candidates: &'static [&'static str],
}

const DEFAULT_FONT_FALLBACKS: &[&str] = &[
    r"C:\Windows\Fonts\segoeui.ttf",
    r"C:\Windows\Fonts\arial.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    "/System/Library/Fonts/Supplemental/Arial.ttf",
];

pub fn save_ration_export(request: ExportCommandRequest) -> Result<ExportResponse> {
    let extension = match request.format.as_str() {
        "pdf" => "pdf",
        "xlsx" => "xlsx",
        "csv" => "csv",
        other => return Err(anyhow!("Unsupported export format: {}", other)),
    };

    let output_dir = resolve_output_dir(&request.options)?;
    let base_name = build_filename(
        request.options.file_name.as_deref(),
        &request.report.title,
        extension,
    );
    let output_path = unique_path(output_dir.join(base_name));

    let appearance = appearance_profile(request.options.appearance.as_deref());
    let font = resolve_font_spec(request.options.font_family.as_deref());

    match request.format.as_str() {
        "pdf" => render_pdf(&request.report, &output_path, appearance, font)?,
        "xlsx" => render_xlsx(&request.report, &output_path, appearance, font)?,
        "csv" => render_csv(&request.report, &output_path)?,
        _ => unreachable!(),
    }

    Ok(ExportResponse {
        path: output_path.display().to_string(),
    })
}

fn resolve_output_dir(options: &ExportOptions) -> Result<PathBuf> {
    if let Some(destination) = options
        .destination_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let path = PathBuf::from(destination);
        fs::create_dir_all(&path)?;
        return Ok(path);
    }

    ensure_export_dir()
}

fn ensure_export_dir() -> Result<PathBuf> {
    let base = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .ok_or_else(|| anyhow!("Could not determine user home directory"))?;

    let documents = if base.join("Documents").exists() {
        base.join("Documents")
    } else {
        base
    };

    let export_dir = documents.join("Felex Exports");
    fs::create_dir_all(&export_dir)?;
    Ok(export_dir)
}

fn build_filename(file_name: Option<&str>, title: &str, extension: &str) -> String {
    let provided_name = file_name.map(str::trim).filter(|value| !value.is_empty());
    let base = provided_name.unwrap_or(title);

    let stem_source = Path::new(base)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(base);
    let sanitized = sanitize_filename(stem_source);
    let stem = if sanitized.is_empty() {
        "ration_report".to_string()
    } else {
        sanitized
    };

    if provided_name.is_some() {
        format!("{}.{}", stem, extension)
    } else {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        format!("{}_{}.{}", stem, timestamp, extension)
    }
}

fn sanitize_filename(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() {
                ch
            } else if ch == ' ' || ch == '-' || ch == '_' {
                '_'
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("report");
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("dat");
    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    for index in 1..1000 {
        let candidate = parent.join(format!("{}_{}.{}", stem, index, ext));
        if !candidate.exists() {
            return candidate;
        }
    }

    path
}

fn appearance_profile(value: Option<&str>) -> AppearanceProfile {
    match value
        .unwrap_or("standard")
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-")
        .as_str()
    {
        "compact" => AppearanceProfile {
            title_size: 16.0,
            section_size: 12.5,
            body_size: 9.2,
            line_height_mm: 5.3,
            wrapped_line_height_mm: 4.8,
            max_chars: 108,
            xlsx_row_height: 21.0,
            xlsx_column_scale: 0.92,
            xlsx_heading_bg: "#E2E8F0",
            xlsx_heading_fg: "#0F172A",
        },
        "presentation" => AppearanceProfile {
            title_size: 21.0,
            section_size: 15.5,
            body_size: 11.0,
            line_height_mm: 6.7,
            wrapped_line_height_mm: 6.2,
            max_chars: 88,
            xlsx_row_height: 28.0,
            xlsx_column_scale: 1.08,
            xlsx_heading_bg: "#C7D2FE",
            xlsx_heading_fg: "#1E1B4B",
        },
        _ => AppearanceProfile {
            title_size: 18.0,
            section_size: 14.0,
            body_size: 10.0,
            line_height_mm: 6.0,
            wrapped_line_height_mm: 5.6,
            max_chars: 96,
            xlsx_row_height: 24.0,
            xlsx_column_scale: 1.0,
            xlsx_heading_bg: "#DBEAFE",
            xlsx_heading_fg: "#1E3A8A",
        },
    }
}

fn resolve_font_spec(value: Option<&str>) -> ExportFontSpec {
    match value
        .unwrap_or("sans")
        .trim()
        .to_ascii_lowercase()
        .replace('_', " ")
        .replace('-', " ")
        .as_str()
    {
        "serif" | "times" | "times new roman" => ExportFontSpec {
            xlsx_name: "Times New Roman",
            pdf_candidates: &[
                r"C:\Windows\Fonts\times.ttf",
                r"C:\Windows\Fonts\timesbd.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
                "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
            ],
        },
        "mono" | "monospace" | "consolas" | "courier" | "courier new" => ExportFontSpec {
            xlsx_name: "Consolas",
            pdf_candidates: &[
                r"C:\Windows\Fonts\consola.ttf",
                r"C:\Windows\Fonts\cour.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
                "/System/Library/Fonts/Supplemental/Courier New.ttf",
            ],
        },
        _ => ExportFontSpec {
            xlsx_name: "Segoe UI",
            pdf_candidates: &[
                r"C:\Windows\Fonts\segoeui.ttf",
                r"C:\Windows\Fonts\arial.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            ],
        },
    }
}

fn resolve_pdf_font_path(font: ExportFontSpec) -> Option<PathBuf> {
    font.pdf_candidates
        .iter()
        .chain(DEFAULT_FONT_FALLBACKS.iter())
        .map(PathBuf::from)
        .find(|path| path.exists())
}

fn escape_csv(value: &str) -> String {
    if value.contains(';') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn render_csv(report: &ExportReport, output_path: &Path) -> Result<()> {
    let mut lines = Vec::new();
    lines.push(escape_csv(&report.title));
    if let Some(subtitle) = report.subtitle.as_deref() {
        if !subtitle.trim().is_empty() {
            lines.push(escape_csv(subtitle));
        }
    }
    lines.push(format!("Дата;{}", escape_csv(&report.generated_at)));
    lines.push(format!("Число голов;{}", report.animal_count));
    lines.push(String::new());

    lines.push("Состав рациона".to_string());
    if report.include_feed_details {
        lines.push(
            "Корм;Категория;кг/гол./сут;кг/группу/сут;СВ %;₽/т;₽/гол./сут;₽/группу/сут;Фикс."
                .to_string(),
        );
    } else {
        lines.push("Корм;Категория;кг/гол./сут;кг/группу/сут;₽/гол./сут;₽/группу/сут".to_string());
    }

    for item in &report.items {
        if report.include_feed_details {
            lines.push(format!(
                "{};{};{};{};{};{};{};{};{}",
                escape_csv(&item.name),
                escape_csv(&item.category),
                format_value(item.amount_kg_per_head),
                format_value(item.amount_kg_total),
                item.dry_matter_pct
                    .map(format_value)
                    .unwrap_or_default(),
                item.price_per_ton
                    .map(format_value)
                    .unwrap_or_default(),
                format_value(item.cost_per_day_per_head),
                format_value(item.cost_per_day_total),
                if item.is_locked { "Да" } else { "Нет" },
            ));
        } else {
            lines.push(format!(
                "{};{};{};{};{};{}",
                escape_csv(&item.name),
                escape_csv(&item.category),
                format_value(item.amount_kg_per_head),
                format_value(item.amount_kg_total),
                format_value(item.cost_per_day_per_head),
                format_value(item.cost_per_day_total),
            ));
        }
    }

    if report.include_feed_details {
        lines.push(format!(
            "Итого;;{};{};;;{};{};",
            format_value(report.total_kg_per_head),
            format_value(report.total_kg),
            format_value(report.total_cost_per_head),
            format_value(report.total_cost),
        ));
    } else {
        lines.push(format!(
            "Итого;;{};{};{};{}",
            format_value(report.total_kg_per_head),
            format_value(report.total_kg),
            format_value(report.total_cost_per_head),
            format_value(report.total_cost),
        ));
    }

    if report.include_nutrients && !report.nutrient_rows.is_empty() {
        lines.push(String::new());
        lines.push("Питательность".to_string());
        if report.include_norms_comparison {
            lines.push("Показатель;Факт;Ед.;Мин.;Цель;Макс.;Статус".to_string());
        } else {
            lines.push("Показатель;Факт;Ед.;Статус".to_string());
        }

        for row in &report.nutrient_rows {
            if report.include_norms_comparison {
                lines.push(format!(
                    "{};{};{};{};{};{};{}",
                    escape_csv(&row.label),
                    format_value(row.actual),
                    escape_csv(&row.unit),
                    row.norm
                        .as_ref()
                        .and_then(|norm| norm.min)
                        .map(format_value)
                        .unwrap_or_default(),
                    row.norm
                        .as_ref()
                        .and_then(|norm| norm.target)
                        .map(format_value)
                        .unwrap_or_default(),
                    row.norm
                        .as_ref()
                        .and_then(|norm| norm.max)
                        .map(format_value)
                        .unwrap_or_default(),
                    escape_csv(&row.status),
                ));
            } else {
                lines.push(format!(
                    "{};{};{};{}",
                    escape_csv(&row.label),
                    format_value(row.actual),
                    escape_csv(&row.unit),
                    escape_csv(&row.status),
                ));
            }
        }
    }

    if report.include_economics {
        lines.push(String::new());
        lines.push("Экономика".to_string());
        lines.push(format!(
            "Стоимость в сутки на голову;{}",
            format_value(report.total_cost_per_head)
        ));
        lines.push(format!(
            "Стоимость в сутки на группу;{}",
            format_value(report.total_cost)
        ));
        lines.push(format!(
            "Стоимость в месяц на группу;{}",
            format_value(report.total_cost * 30.0)
        ));
        lines.push(format!(
            "Стоимость в год на группу;{}",
            format_value(report.total_cost * 365.0)
        ));
    }

    if !report.notes.trim().is_empty() {
        lines.push(String::new());
        lines.push("Примечания".to_string());
        lines.push(report.notes.clone());
    }

    let mut file = File::create(output_path)?;
    file.write_all("\u{feff}".as_bytes())?;
    file.write_all(lines.join("\n").as_bytes())?;
    Ok(())
}

fn render_xlsx(
    report: &ExportReport,
    output_path: &Path,
    appearance: AppearanceProfile,
    font: ExportFontSpec,
) -> Result<()> {
    let mut workbook = Workbook::new();

    let title_format = Format::new()
        .set_bold()
        .set_font_name(font.xlsx_name)
        .set_font_size(appearance.title_size);
    let subtitle_format = Format::new()
        .set_font_name(font.xlsx_name)
        .set_font_size(appearance.body_size)
        .set_font_color("#475569");
    let heading = Format::new()
        .set_bold()
        .set_font_name(font.xlsx_name)
        .set_font_size(appearance.body_size)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter)
        .set_background_color(appearance.xlsx_heading_bg)
        .set_font_color(appearance.xlsx_heading_fg);
    let body = Format::new()
        .set_font_name(font.xlsx_name)
        .set_font_size(appearance.body_size);
    let body_center = Format::new()
        .set_font_name(font.xlsx_name)
        .set_font_size(appearance.body_size)
        .set_align(FormatAlign::Center);
    let body_wrap = Format::new()
        .set_font_name(font.xlsx_name)
        .set_font_size(appearance.body_size)
        .set_text_wrap();
    let numeric = Format::new()
        .set_font_name(font.xlsx_name)
        .set_font_size(appearance.body_size)
        .set_num_format("0.00");
    let currency = Format::new()
        .set_font_name(font.xlsx_name)
        .set_font_size(appearance.body_size)
        .set_num_format("#,##0.00");
    let total_format = Format::new()
        .set_bold()
        .set_font_name(font.xlsx_name)
        .set_font_size(appearance.body_size)
        .set_background_color(Color::RGB(0xE5E7EB));

    {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("Рацион")?;
        worksheet.set_column_width(0, 28.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(1, 18.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(2, 14.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(3, 14.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(4, 12.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(5, 14.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(6, 14.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(7, 16.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(8, 12.0 * appearance.xlsx_column_scale)?;
        worksheet.set_row_height(0, appearance.xlsx_row_height)?;

        worksheet.write_with_format(0, 0, &report.title, &title_format)?;
        if let Some(subtitle) = report.subtitle.as_deref() {
            if !subtitle.trim().is_empty() {
                worksheet.write_with_format(1, 0, subtitle, &subtitle_format)?;
            }
        }
        worksheet.write_with_format(2, 0, &format!("Дата: {}", report.generated_at), &subtitle_format)?;
        worksheet.write_with_format(
            3,
            0,
            &format!("Число голов: {}", report.animal_count),
            &subtitle_format,
        )?;

        worksheet.write_with_format(5, 0, "Корм", &heading)?;
        worksheet.write_with_format(5, 1, "Категория", &heading)?;
        worksheet.write_with_format(5, 2, "кг/гол./сут", &heading)?;
        worksheet.write_with_format(5, 3, "кг/группу/сут", &heading)?;
        if report.include_feed_details {
            worksheet.write_with_format(5, 4, "СВ %", &heading)?;
            worksheet.write_with_format(5, 5, "₽/т", &heading)?;
        }
        worksheet.write_with_format(5, 6, "₽/гол./сут", &heading)?;
        worksheet.write_with_format(5, 7, "₽/группу/сут", &heading)?;
        if report.include_feed_details {
            worksheet.write_with_format(5, 8, "Фикс.", &heading)?;
        }

        let mut row = 6u32;
        for item in &report.items {
            worksheet.write_with_format(row, 0, &item.name, &body_wrap)?;
            worksheet.write_with_format(row, 1, &item.category, &body)?;
            worksheet.write_with_format(row, 2, item.amount_kg_per_head, &numeric)?;
            worksheet.write_with_format(row, 3, item.amount_kg_total, &numeric)?;
            if report.include_feed_details {
                if let Some(value) = item.dry_matter_pct {
                    worksheet.write_with_format(row, 4, value, &numeric)?;
                }
                if let Some(value) = item.price_per_ton {
                    worksheet.write_with_format(row, 5, value, &currency)?;
                }
            }
            worksheet.write_with_format(row, 6, item.cost_per_day_per_head, &currency)?;
            worksheet.write_with_format(row, 7, item.cost_per_day_total, &currency)?;
            if report.include_feed_details {
                worksheet.write_with_format(
                    row,
                    8,
                    if item.is_locked { "Да" } else { "Нет" },
                    &body_center,
                )?;
            }
            row += 1;
        }

        worksheet.write_with_format(row, 0, "Итого", &total_format)?;
        worksheet.write_with_format(row, 2, report.total_kg_per_head, &total_format)?;
        worksheet.write_with_format(row, 3, report.total_kg, &total_format)?;
        worksheet.write_with_format(row, 6, report.total_cost_per_head, &total_format)?;
        worksheet.write_with_format(row, 7, report.total_cost, &total_format)?;
    }

    if report.include_nutrients && !report.nutrient_rows.is_empty() {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("Питательность")?;
        worksheet.set_column_width(0, 28.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(1, 14.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(2, 10.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(3, 12.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(4, 12.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(5, 12.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(6, 16.0 * appearance.xlsx_column_scale)?;
        worksheet.set_row_height(0, appearance.xlsx_row_height)?;

        worksheet.write_with_format(0, 0, "Показатель", &heading)?;
        worksheet.write_with_format(0, 1, "Факт", &heading)?;
        worksheet.write_with_format(0, 2, "Ед.", &heading)?;
        if report.include_norms_comparison {
            worksheet.write_with_format(0, 3, "Мин.", &heading)?;
            worksheet.write_with_format(0, 4, "Цель", &heading)?;
            worksheet.write_with_format(0, 5, "Макс.", &heading)?;
            worksheet.write_with_format(0, 6, "Статус", &heading)?;
        } else {
            worksheet.write_with_format(0, 3, "Статус", &heading)?;
        }

        for (index, nutrient) in report.nutrient_rows.iter().enumerate() {
            let row = (index + 1) as u32;
            worksheet.write_with_format(row, 0, &nutrient.label, &body_wrap)?;
            worksheet.write_with_format(row, 1, nutrient.actual, &numeric)?;
            worksheet.write_with_format(row, 2, &nutrient.unit, &body)?;
            if report.include_norms_comparison {
                if let Some(norm) = &nutrient.norm {
                    if let Some(value) = norm.min {
                        worksheet.write_with_format(row, 3, value, &numeric)?;
                    }
                    if let Some(value) = norm.target {
                        worksheet.write_with_format(row, 4, value, &numeric)?;
                    }
                    if let Some(value) = norm.max {
                        worksheet.write_with_format(row, 5, value, &numeric)?;
                    }
                }
                worksheet.write_with_format(row, 6, &nutrient.status, &body)?;
            } else {
                worksheet.write_with_format(row, 3, &nutrient.status, &body)?;
            }
        }
    }

    if report.include_economics {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name("Экономика")?;
        worksheet.set_column_width(0, 28.0 * appearance.xlsx_column_scale)?;
        worksheet.set_column_width(1, 18.0 * appearance.xlsx_column_scale)?;
        worksheet.set_row_height(0, appearance.xlsx_row_height)?;

        worksheet.write_with_format(0, 0, "Показатель", &heading)?;
        worksheet.write_with_format(0, 1, "Значение", &heading)?;
        worksheet.write_with_format(1, 0, "Стоимость в сутки на голову", &body)?;
        worksheet.write_with_format(1, 1, report.total_cost_per_head, &currency)?;
        worksheet.write_with_format(2, 0, "Стоимость в сутки на группу", &body)?;
        worksheet.write_with_format(2, 1, report.total_cost, &currency)?;
        worksheet.write_with_format(3, 0, "Стоимость в месяц на группу", &body)?;
        worksheet.write_with_format(3, 1, report.total_cost * 30.0, &currency)?;
        worksheet.write_with_format(4, 0, "Стоимость в год на группу", &body)?;
        worksheet.write_with_format(4, 1, report.total_cost * 365.0, &currency)?;
        worksheet.write_with_format(6, 0, "Масса рациона на голову", &body)?;
        worksheet.write_with_format(6, 1, report.total_kg_per_head, &numeric)?;
        worksheet.write_with_format(7, 0, "Масса рациона на группу", &body)?;
        worksheet.write_with_format(7, 1, report.total_kg, &numeric)?;

        if !report.notes.trim().is_empty() {
            worksheet.write_with_format(9, 0, "Примечания", &heading)?;
            worksheet.write_with_format(10, 0, &report.notes, &body_wrap)?;
        }
    }

    workbook.save(output_path)?;
    Ok(())
}

struct PdfState {
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    y_mm: f32,
}

fn render_pdf(
    report: &ExportReport,
    output_path: &Path,
    appearance: AppearanceProfile,
    font_spec: ExportFontSpec,
) -> Result<()> {
    let (doc, page, layer) = PdfDocument::new(&report.title, Mm(210.0), Mm(297.0), "Felex report");
    let font_path = resolve_pdf_font_path(font_spec)
        .ok_or_else(|| anyhow!("Could not find a Unicode font for PDF export"))?;
    let font = doc
        .add_external_font(
            File::open(&font_path)
                .with_context(|| format!("Could not open font {}", font_path.display()))?,
        )
        .context("Could not load PDF font")?;

    let mut state = PdfState { page, layer, y_mm: 282.0 };

    write_line_internal(
        &doc,
        &font,
        &mut state,
        &report.title,
        appearance.title_size,
        appearance.line_height_mm + 2.0,
    )?;
    if let Some(subtitle) = report.subtitle.as_deref() {
        if !subtitle.trim().is_empty() {
            write_line_internal(
                &doc,
                &font,
                &mut state,
                subtitle,
                appearance.body_size,
                appearance.line_height_mm,
            )?;
        }
    }
    write_line_internal(
        &doc,
        &font,
        &mut state,
        &format!("Дата: {}", report.generated_at),
        appearance.body_size,
        appearance.line_height_mm,
    )?;
    write_line_internal(
        &doc,
        &font,
        &mut state,
        &format!("Число голов: {}", report.animal_count),
        appearance.body_size,
        appearance.line_height_mm,
    )?;
    blank_line(&mut state);

    write_line_internal(
        &doc,
        &font,
        &mut state,
        "Состав рациона",
        appearance.section_size,
        appearance.line_height_mm + 1.0,
    )?;
    for item in &report.items {
        let line = if report.include_feed_details {
            format!(
                "{} | {} | {:.2} кг/гол. | {:.2} кг/гр. | СВ {} | ₽/т {} | ₽/гол. {:.2} | ₽/гр. {:.2} | {}",
                item.name,
                item.category,
                item.amount_kg_per_head,
                item.amount_kg_total,
                item.dry_matter_pct.map(format_value).unwrap_or_else(|| "-".to_string()),
                item.price_per_ton.map(format_value).unwrap_or_else(|| "-".to_string()),
                item.cost_per_day_per_head,
                item.cost_per_day_total,
                if item.is_locked { "фикс." } else { "свободно" },
            )
        } else {
            format!(
                "{} | {} | {:.2} кг/гол. | {:.2} кг/гр. | ₽/гол. {:.2} | ₽/гр. {:.2}",
                item.name,
                item.category,
                item.amount_kg_per_head,
                item.amount_kg_total,
                item.cost_per_day_per_head,
                item.cost_per_day_total,
            )
        };

        write_wrapped_line(
            &doc,
            &font,
            &mut state,
            &line,
            appearance.body_size,
            appearance.wrapped_line_height_mm,
            appearance.max_chars,
        )?;
    }
    write_line_internal(
        &doc,
        &font,
        &mut state,
        &format!(
            "Итого: {:.2} кг/гол. | {:.2} кг/группу | {:.2} ₽/гол. | {:.2} ₽/группу",
            report.total_kg_per_head,
            report.total_kg,
            report.total_cost_per_head,
            report.total_cost,
        ),
        appearance.body_size,
        appearance.line_height_mm,
    )?;

    if report.include_nutrients && !report.nutrient_rows.is_empty() {
        blank_line(&mut state);
        write_line_internal(
            &doc,
            &font,
            &mut state,
            "Питательность",
            appearance.section_size,
            appearance.line_height_mm + 1.0,
        )?;
        for nutrient in &report.nutrient_rows {
            let line = if report.include_norms_comparison {
                let norm_text = if let Some(norm) = &nutrient.norm {
                    format!(
                        "мин {} | цель {} | макс {}",
                        norm.min.map(format_value).unwrap_or_else(|| "-".to_string()),
                        norm.target.map(format_value).unwrap_or_else(|| "-".to_string()),
                        norm.max.map(format_value).unwrap_or_else(|| "-".to_string()),
                    )
                } else {
                    "норма не задана".to_string()
                };

                format!(
                    "{}: {} {} | {} | {}",
                    nutrient.label,
                    format_value(nutrient.actual),
                    nutrient.unit,
                    norm_text,
                    nutrient.status,
                )
            } else {
                format!(
                    "{}: {} {} | {}",
                    nutrient.label,
                    format_value(nutrient.actual),
                    nutrient.unit,
                    nutrient.status,
                )
            };

            write_wrapped_line(
                &doc,
                &font,
                &mut state,
                &line,
                appearance.body_size,
                appearance.wrapped_line_height_mm,
                appearance.max_chars,
            )?;
        }
    }

    if report.include_economics {
        blank_line(&mut state);
        write_line_internal(
            &doc,
            &font,
            &mut state,
            "Экономика",
            appearance.section_size,
            appearance.line_height_mm + 1.0,
        )?;
        write_line_internal(
            &doc,
            &font,
            &mut state,
            &format!("Стоимость в сутки на голову: {:.2} ₽", report.total_cost_per_head),
            appearance.body_size,
            appearance.line_height_mm,
        )?;
        write_line_internal(
            &doc,
            &font,
            &mut state,
            &format!("Стоимость в сутки на группу: {:.2} ₽", report.total_cost),
            appearance.body_size,
            appearance.line_height_mm,
        )?;
        write_line_internal(
            &doc,
            &font,
            &mut state,
            &format!("Стоимость в месяц на группу: {:.2} ₽", report.total_cost * 30.0),
            appearance.body_size,
            appearance.line_height_mm,
        )?;
        write_line_internal(
            &doc,
            &font,
            &mut state,
            &format!("Стоимость в год на группу: {:.2} ₽", report.total_cost * 365.0),
            appearance.body_size,
            appearance.line_height_mm,
        )?;
    }

    if !report.notes.trim().is_empty() {
        blank_line(&mut state);
        write_line_internal(
            &doc,
            &font,
            &mut state,
            "Примечания",
            appearance.section_size,
            appearance.line_height_mm + 1.0,
        )?;
        write_wrapped_line(
            &doc,
            &font,
            &mut state,
            &report.notes,
            appearance.body_size,
            appearance.wrapped_line_height_mm,
            appearance.max_chars,
        )?;
    }

    let mut writer = BufWriter::new(File::create(output_path)?);
    doc.save(&mut writer)?;
    Ok(())
}

fn write_wrapped_line(
    doc: &PdfDocumentReference,
    font: &IndirectFontRef,
    state: &mut PdfState,
    text: &str,
    size: f32,
    line_height_mm: f32,
    max_chars: usize,
) -> Result<()> {
    for line in wrap_text(text, max_chars) {
        write_line_internal(doc, font, state, &line, size, line_height_mm)?;
    }
    Ok(())
}

fn write_line_internal(
    doc: &PdfDocumentReference,
    font: &IndirectFontRef,
    state: &mut PdfState,
    text: &str,
    size: f32,
    line_height_mm: f32,
) -> Result<()> {
    ensure_pdf_space(doc, state, line_height_mm);
    let layer = doc.get_page(state.page).get_layer(state.layer);
    layer.use_text(text, size, Mm(14.0), Mm(state.y_mm), font);
    state.y_mm -= line_height_mm;
    Ok(())
}

fn blank_line(state: &mut PdfState) {
    state.y_mm -= 4.0;
}

fn ensure_pdf_space(doc: &PdfDocumentReference, state: &mut PdfState, required_mm: f32) {
    if state.y_mm > 18.0 + required_mm {
        return;
    }

    let (page, layer) = doc.add_page(Mm(210.0), Mm(297.0), "Felex report");
    state.page = page;
    state.layer = layer;
    state.y_mm = 282.0;
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for raw_line in text.lines() {
        if raw_line.trim().is_empty() {
            if !current.is_empty() {
                lines.push(current.trim_end().to_string());
                current.clear();
            }
            lines.push(String::new());
            continue;
        }

        for word in raw_line.split_whitespace() {
            if current.is_empty() {
                current.push_str(word);
                continue;
            }

            if current.chars().count() + 1 + word.chars().count() > max_chars {
                lines.push(current.trim_end().to_string());
                current.clear();
                current.push_str(word);
            } else {
                current.push(' ');
                current.push_str(word);
            }
        }

        if !current.is_empty() {
            lines.push(current.trim_end().to_string());
            current.clear();
        }
    }

    if !current.is_empty() {
        lines.push(current.trim_end().to_string());
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn format_value(value: f64) -> String {
    if value.abs() >= 100.0 {
        format!("{:.0}", value)
    } else if value.abs() >= 10.0 {
        format!("{:.1}", value)
    } else {
        format!("{:.2}", value)
    }
}
