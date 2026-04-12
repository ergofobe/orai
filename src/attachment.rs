use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

const TEXT_EXTENSIONS: &[&str] = &[
    "md",
    "txt",
    "rs",
    "py",
    "js",
    "ts",
    "jsx",
    "tsx",
    "json",
    "yaml",
    "yml",
    "toml",
    "html",
    "css",
    "scss",
    "sh",
    "bash",
    "zsh",
    "fish",
    "go",
    "java",
    "c",
    "cpp",
    "h",
    "hpp",
    "rb",
    "sql",
    "xml",
    "csv",
    "log",
    "cfg",
    "ini",
    "conf",
    "env",
    "gitignore",
    "dockerignore",
    "editorconfig",
    "makefile",
    "cmake",
    "gradle",
    "proto",
    "zig",
    "nim",
    "lua",
    "r",
    "R",
    "pl",
    "pm",
    "ex",
    "exs",
    "erl",
    "hs",
    "ml",
    "mli",
    "v",
    "vhd",
    "asm",
    "s",
    "nix",
    "lock",
    "ipynb",
];

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg"];

#[derive(Debug, Clone)]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { url: String },
}

impl ContentPart {
    pub fn to_openrouter(&self) -> Value {
        match self {
            ContentPart::Text { text } => serde_json::json!({
                "type": "text",
                "text": text
            }),
            ContentPart::ImageUrl { url } => serde_json::json!({
                "type": "image_url",
                "image_url": { "url": url }
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub filename: String,
    pub parts: Vec<ContentPart>,
}

pub fn load_attachment(path: &str) -> Result<Attachment> {
    let p = Path::new(path);
    if !p.exists() {
        bail!("File not found: {}", path);
    }

    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let filename = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
        .to_string();

    let parts = if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        load_image(p)?
    } else if ext == "pdf" {
        load_pdf(p)?
    } else if TEXT_EXTENSIONS.contains(&ext.as_str()) {
        load_text(p)?
    } else {
        load_as_image_fallback(p)?
    };

    Ok(Attachment { filename, parts })
}

fn load_image(path: &Path) -> Result<Vec<ContentPart>> {
    let bytes = std::fs::read(path).context("Failed to read image file")?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let b64 = BASE64.encode(&bytes);
    let url = format!("data:{};base64,{}", mime, b64);
    Ok(vec![ContentPart::ImageUrl { url }])
}

fn load_text(path: &Path) -> Result<Vec<ContentPart>> {
    let content = std::fs::read_to_string(path).context("Failed to read text file")?;
    let header = format!("--- Attachment: {} ---", path.display());
    let footer = "--- End Attachment ---";
    Ok(vec![ContentPart::Text {
        text: format!("{}\n{}\n{}", header, content, footer),
    }])
}

fn load_pdf(path: &Path) -> Result<Vec<ContentPart>> {
    if let Ok(pages) = render_pdf_with_pdftoppm(path) {
        return Ok(pages);
    }

    if let Ok(pages) = render_pdf_with_imagemagick(path) {
        return Ok(pages);
    }

    eprintln!(
        "Warning: PDF rendered as text only. Install poppler-utils or ImageMagick for image support."
    );
    extract_pdf_text(path)
}

fn render_pdf_with_pdftoppm(path: &Path) -> Result<Vec<ContentPart>> {
    let output = Command::new("pdftoppm")
        .arg("-png")
        .arg("-r")
        .arg("150")
        .arg("-singlefile")
        .arg(path)
        .arg("/tmp/orai_pdf_page")
        .output()?;

    if !output.status.success() {
        bail!("pdftoppm failed");
    }

    let mut parts = Vec::new();
    let mut page_num = 1;
    loop {
        let page_path_str = format!("/tmp/orai_pdf_page-{}.png", page_num);
        let page_path = Path::new(&page_path_str);
        if !page_path.exists() {
            if page_num == 1 {
                let single = Path::new("/tmp/orai_pdf_page.png");
                if single.exists() {
                    parts.push(ContentPart::Text {
                        text: format!("--- PDF Page 1 of {} ---", path.display()),
                    });
                    let bytes = std::fs::read(single)?;
                    let b64 = BASE64.encode(&bytes);
                    parts.push(ContentPart::ImageUrl {
                        url: format!("data:image/png;base64,{}", b64),
                    });
                    let _ = std::fs::remove_file(single);
                    break;
                }
            }
            break;
        }
        parts.push(ContentPart::Text {
            text: format!("--- PDF Page {} of {} ---", page_num, path.display()),
        });
        let bytes = std::fs::read(page_path)?;
        let b64 = BASE64.encode(&bytes);
        parts.push(ContentPart::ImageUrl {
            url: format!("data:image/png;base64,{}", b64),
        });
        let _ = std::fs::remove_file(page_path);
        page_num += 1;
    }

    if parts.is_empty() {
        bail!("pdftoppm produced no output");
    }
    Ok(parts)
}

fn render_pdf_with_imagemagick(path: &Path) -> Result<Vec<ContentPart>> {
    let output = Command::new("convert")
        .arg("-density")
        .arg("150")
        .arg(path)
        .arg("/tmp/orai_pdf_page.png")
        .output()?;

    if !output.status.success() {
        bail!("ImageMagick convert failed");
    }

    let mut parts = Vec::new();
    let mut page_num = 1;
    loop {
        let page_path = if page_num == 1 {
            Path::new("/tmp/orai_pdf_page.png").to_path_buf()
        } else {
            Path::new(&format!("/tmp/orai_pdf_page-{}.png", page_num)).to_path_buf()
        };

        if !page_path.exists() {
            break;
        }

        parts.push(ContentPart::Text {
            text: format!("--- PDF Page {} of {} ---", page_num, path.display()),
        });
        let bytes = std::fs::read(&page_path)?;
        let b64 = BASE64.encode(&bytes);
        parts.push(ContentPart::ImageUrl {
            url: format!("data:image/png;base64,{}", b64),
        });
        let _ = std::fs::remove_file(&page_path);
        page_num += 1;
    }

    if parts.is_empty() {
        bail!("ImageMagick produced no output");
    }
    Ok(parts)
}

fn extract_pdf_text(path: &Path) -> Result<Vec<ContentPart>> {
    let doc = lopdf::Document::load(path).context("Failed to load PDF with lopdf")?;
    let mut parts = Vec::new();
    let mut all_text = String::new();

    let pages = doc.get_pages();
    for (page_num, page_id) in &pages {
        match doc.get_page_content(*page_id) {
            Ok(content_bytes) => {
                if let Ok(content_str) = String::from_utf8(content_bytes) {
                    let page_text = extract_text_from_content(&content_str);
                    all_text.push_str(&format!("--- Page {} ---\n{}\n\n", page_num, page_text));
                }
            }
            Err(_) => continue,
        }
    }

    if all_text.is_empty() {
        all_text = format!("(No text could be extracted from PDF: {})", path.display());
    }

    parts.push(ContentPart::Text { text: all_text });
    Ok(parts)
}

fn extract_text_from_content(content: &str) -> String {
    let mut text = String::new();
    for line in content.lines() {
        if line.starts_with('(') && line.ends_with(')') {
            let s = &line[1..line.len() - 1];
            text.push_str(s);
            text.push(' ');
        }
    }
    text
}

fn load_as_image_fallback(path: &Path) -> Result<Vec<ContentPart>> {
    let bytes = std::fs::read(path).context("Failed to read file")?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let b64 = BASE64.encode(&bytes);
    let url = format!("data:{};base64,{}", mime, b64);
    Ok(vec![ContentPart::ImageUrl { url }])
}

pub fn parse_attachments_from_text(text: &str) -> (String, Vec<String>) {
    let re = regex::Regex::new(r"\+([\w./-]+\.\w{1,10})").unwrap();
    let mut clean_text = text.to_string();
    let mut attachments = Vec::new();

    for cap in re.captures_iter(text) {
        let filename = cap[1].to_string();
        attachments.push(filename.clone());
    }

    clean_text = re.replace_all(&clean_text, "").trim().to_string();
    (clean_text, attachments)
}
