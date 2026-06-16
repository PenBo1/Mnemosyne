use crate::errors::AppError;
use std::io::Write;
use std::path::Path;
use zip::write::FileOptions;
use zip::CompressionMethod;

/// Export book chapters to EPUB format
pub async fn export_epub(
    book_dir: &Path,
    output_path: &Path,
    title: &str,
    author: &str,
) -> Result<(), AppError> {
    let chapters_dir = book_dir.join("chapters");
    if !chapters_dir.exists() {
        return Err(AppError::not_found("No chapters directory found"));
    }

    let mut chapter_files: Vec<String> = std::fs::read_dir(&chapters_dir)
        .map_err(|e| AppError::internal(format!("Failed to read chapters: {}", e)))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|f| f.ends_with(".md") && !f.starts_with("index"))
        .collect();
    chapter_files.sort();

    if chapter_files.is_empty() {
        return Err(AppError::not_found("No chapter files found"));
    }

    let mut chapters_xhtml = Vec::new();
    let mut toc_entries = Vec::new();

    for (i, filename) in chapter_files.iter().enumerate() {
        let content = std::fs::read_to_string(chapters_dir.join(filename))
            .map_err(|e| AppError::internal(format!("Failed to read chapter {}: {}", filename, e)))?;

        let (chapter_title, chapter_body) = parse_chapter_md(&content);
        let chapter_id = format!("chapter-{}", i + 1);
        let xhtml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>{}</title></head>
<body>
<h1>{}</h1>
{}
</body>
</html>"#,
            escape_xml(&chapter_title),
            escape_xml(&chapter_title),
            markdown_to_xhtml(&chapter_body)
        );
        chapters_xhtml.push((chapter_id.clone(), xhtml));
        toc_entries.push((chapter_id, chapter_title));
    }

    let manifest_items: Vec<String> = toc_entries.iter().map(|(id, _)| {
        format!("    <item id=\"{}\" href=\"{}.xhtml\" media-type=\"application/xhtml+xml\"/>", id, id)
    }).collect();

    let spine_items: Vec<String> = toc_entries.iter().map(|(id, _)| {
        format!("    <itemref idref=\"{}\"/>", id)
    }).collect();

    let nav_items: Vec<String> = toc_entries.iter().map(|(id, title)| {
        format!("      <li><a href=\"{}.xhtml\">{}</a></li>", id, escape_xml(title))
    }).collect();

    let opf = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="bookid" version="3.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>{}</dc:title>
    <dc:creator>{}</dc:creator>
    <dc:language>zh</dc:language>
    <dc:identifier id="bookid">urn:uuid:{}</dc:identifier>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
{}
  </manifest>
  <spine>
{}
  </spine>
</package>"#,
        escape_xml(title), escape_xml(author), uuid::Uuid::new_v4(),
        manifest_items.join("\n"), spine_items.join("\n")
    );

    let nav_xhtml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head><title>Table of Contents</title></head>
<body>
<h1>Table of Contents</h1>
<nav epub:type="toc">
  <ol>
{}
  </ol>
</nav>
</body>
</html>"#,
        nav_items.join("\n")
    );

    // Write EPUB as ZIP
    let epub_path = output_path.with_extension("epub");
    let zip_file = std::fs::File::create(&epub_path)
        .map_err(|e| AppError::internal(format!("Failed to create EPUB: {}", e)))?;
    let mut zip = zip::ZipWriter::new(zip_file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated);

    // mimetype (must be first, uncompressed)
    let stored_options = FileOptions::default()
        .compression_method(CompressionMethod::Stored);
    zip.start_file("mimetype", stored_options)
        .map_err(|e| AppError::internal(format!("ZIP error: {}", e)))?;
    zip.write_all(b"application/epub+zip")
        .map_err(|e| AppError::internal(format!("ZIP write error: {}", e)))?;

    // META-INF/container.xml
    zip.start_file("META-INF/container.xml", options)
        .map_err(|e| AppError::internal(format!("ZIP error: {}", e)))?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8"?>
<container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#).map_err(|e| AppError::internal(format!("ZIP write error: {}", e)))?;

    // content.opf
    zip.start_file("OEBPS/content.opf", options)
        .map_err(|e| AppError::internal(format!("ZIP error: {}", e)))?;
    zip.write_all(opf.as_bytes())
        .map_err(|e| AppError::internal(format!("ZIP write error: {}", e)))?;

    // nav.xhtml
    zip.start_file("OEBPS/nav.xhtml", options)
        .map_err(|e| AppError::internal(format!("ZIP error: {}", e)))?;
    zip.write_all(nav_xhtml.as_bytes())
        .map_err(|e| AppError::internal(format!("ZIP write error: {}", e)))?;

    // Chapter XHTML files
    for (id, xhtml) in &chapters_xhtml {
        zip.start_file(format!("OEBPS/{}.xhtml", id), options)
            .map_err(|e| AppError::internal(format!("ZIP error: {}", e)))?;
        zip.write_all(xhtml.as_bytes())
            .map_err(|e| AppError::internal(format!("ZIP write error: {}", e)))?;
    }

    zip.finish()
        .map_err(|e| AppError::internal(format!("ZIP finish error: {}", e)))?;

    tracing::info!(path = %epub_path.display(), chapters = chapter_files.len(), "EPUB exported");
    Ok(())
}

fn parse_chapter_md(content: &str) -> (String, String) {
    let lines: Vec<&str> = content.lines().collect();
    let mut title = String::new();
    let mut body_start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            title = stripped.to_string();
            body_start = i + 1;
            break;
        }
    }

    if title.is_empty() {
        title = "Untitled".to_string();
    }

    let body = lines[body_start..].join("\n");
    (title, body)
}

fn markdown_to_xhtml(md: &str) -> String {
    let mut html = String::new();
    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            html.push_str("<br/>\n");
        } else if let Some(stripped) = trimmed.strip_prefix("## ") {
            html.push_str(&format!("<h2>{}</h2>\n", escape_xml(stripped)));
        } else if let Some(stripped) = trimmed.strip_prefix("### ") {
            html.push_str(&format!("<h3>{}</h3>\n", escape_xml(stripped)));
        } else if let Some(stripped) = trimmed.strip_prefix("- ") {
            html.push_str(&format!("<p>• {}</p>\n", escape_xml(stripped)));
        } else {
            html.push_str(&format!("<p>{}</p>\n", escape_xml(trimmed)));
        }
    }
    html
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
