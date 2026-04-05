use anyhow::{anyhow, Context, Result};
use printpdf::{IndirectFontRef, Mm, PdfDocument, PdfDocumentReference, PdfLayerIndex, PdfPageIndex};
use std::fs::File;
use std::io::{BufWriter, Read};
use std::path::{Path, PathBuf};

const FONT_FALLBACKS: &[&str] = &[
    r"C:\Windows\Fonts\segoeui.ttf",
    r"C:\Windows\Fonts\arial.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    "/System/Library/Fonts/Supplemental/Arial.ttf",
];

struct DocSpec {
    input: &'static str,
    output: &'static str,
    title: &'static str,
}

struct RenderState {
    page: PdfPageIndex,
    layer: PdfLayerIndex,
    y_mm: f32,
}

fn main() -> Result<()> {
    let root = std::env::current_dir().context("Could not determine current directory")?;
    let docs = [
        DocSpec {
            input: "frontend/public/docs/technical-documentation.ru.md",
            output: "frontend/public/docs/technical-documentation.ru.pdf",
            title: "Felex technical documentation (RU)",
        },
        DocSpec {
            input: "frontend/public/docs/technical-documentation.en.md",
            output: "frontend/public/docs/technical-documentation.en.pdf",
            title: "Felex technical documentation (EN)",
        },
    ];

    for doc in docs {
        let input_path = root.join(doc.input);
        let output_path = root.join(doc.output);
        let markdown = read_utf8(&input_path)
            .with_context(|| format!("Could not read {}", input_path.display()))?;
        render_markdown_pdf(&markdown, doc.title, &output_path)?;
        println!("Generated {}", output_path.display());
    }

    Ok(())
}

fn read_utf8(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    Ok(buffer.trim_start_matches('\u{feff}').to_string())
}

fn render_markdown_pdf(markdown: &str, title: &str, output_path: &Path) -> Result<()> {
    let (doc, page, layer) = PdfDocument::new(title, Mm(210.0), Mm(297.0), "Help");
    let font_path = resolve_font_path().ok_or_else(|| anyhow!("Could not find a Unicode font"))?;
    let font = doc
        .add_external_font(File::open(&font_path).with_context(|| format!("Could not open font {}", font_path.display()))?)
        .context("Could not load PDF font")?;

    let mut state = RenderState {
        page,
        layer,
        y_mm: 282.0,
    };

    for raw_line in markdown.lines() {
        let trimmed = raw_line.trim_end();

        if trimmed.is_empty() {
            state.y_mm -= 4.0;
            continue;
        }

        let (text, size, step, max_chars) = if let Some(rest) = trimmed.strip_prefix("# ") {
            (rest.trim(), 18.0, 8.0, 60)
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            (rest.trim(), 14.0, 7.0, 72)
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            (rest.trim(), 11.0, 6.0, 86)
        } else if let Some(rest) = trimmed.strip_prefix("- ") {
            (rest.trim(), 10.0, 5.8, 96)
        } else if trimmed.starts_with('|') {
            (trimmed, 9.0, 5.4, 108)
        } else if trimmed.starts_with("```") {
            continue;
        } else {
            (trimmed, 10.0, 5.8, 96)
        };

        let prepared = if trimmed.starts_with("- ") {
            format!("- {}", text)
        } else {
            text.to_string()
        };

        write_wrapped(&doc, &font, &mut state, &prepared, size, step, max_chars)?;
    }

    let mut writer = BufWriter::new(File::create(output_path)?);
    doc.save(&mut writer)?;
    Ok(())
}

fn resolve_font_path() -> Option<PathBuf> {
    FONT_FALLBACKS
        .iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
}

fn write_wrapped(
    doc: &PdfDocumentReference,
    font: &IndirectFontRef,
    state: &mut RenderState,
    text: &str,
    font_size: f32,
    line_step_mm: f32,
    max_chars: usize,
) -> Result<()> {
    for line in wrap_text(text, max_chars) {
        ensure_space(doc, state, line_step_mm);
        let layer = doc.get_page(state.page).get_layer(state.layer);
        layer.use_text(line, font_size, Mm(15.0), Mm(state.y_mm), font);
        state.y_mm -= line_step_mm;
    }
    Ok(())
}

fn ensure_space(doc: &PdfDocumentReference, state: &mut RenderState, line_step_mm: f32) {
    if state.y_mm > 18.0 {
        return;
    }

    let (page, layer) = doc.add_page(Mm(210.0), Mm(297.0), "Help");
    state.page = page;
    state.layer = layer;
    state.y_mm = 282.0 - line_step_mm;
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    if text.chars().count() <= max_chars {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current, word)
        };

        if candidate.chars().count() > max_chars && !current.is_empty() {
            lines.push(current);
            current = word.to_string();
        } else {
            current = candidate;
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}
