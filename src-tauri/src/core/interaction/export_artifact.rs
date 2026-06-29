use crate::shared::errors::AppError;
use std::path::Path;

/// Export format
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportFormat {
    Txt,
    Md,
    Epub,
}

impl ExportFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "txt" => Some(Self::Txt),
            "md" => Some(Self::Md),
            "epub" => Some(Self::Epub),
            _ => None,
        }
    }

    pub fn extension(&self) -> &str {
        match self {
            Self::Txt => "txt",
            Self::Md => "md",
            Self::Epub => "epub",
        }
    }

    pub fn content_type(&self) -> &str {
        match self {
            Self::Txt => "text/plain",
            Self::Md => "text/markdown",
            Self::Epub => "application/epub+zip",
        }
    }
}

/// Export artifact result
pub struct ExportArtifact {
    pub output_path: String,
    pub file_name: String,
    pub chapters_exported: u32,
    pub total_words: u32,
    pub format: ExportFormat,
}

/// Build export artifact from book chapters
pub async fn build_export_artifact(
    book_dir: &Path,
    title: &str,
    format: ExportFormat,
    approved_only: bool,
) -> Result<ExportArtifact, AppError> {
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

    let mut total_words = 0u32;
    let mut content = String::new();

    for filename in &chapter_files {
        let chapter_content = std::fs::read_to_string(chapters_dir.join(filename))
            .map_err(|e| AppError::internal(format!("Failed to read chapter {}: {}", filename, e)))?;

        // Skip if approved_only and not approved
        if approved_only {
            // In a real implementation, check chapter status from index
        }

        let words = chapter_content.split_whitespace().count() as u32;
        total_words += words;

        match format {
            ExportFormat::Txt => {
                content.push_str(&chapter_content);
                content.push_str("\n\n");
            }
            ExportFormat::Md => {
                content.push_str(&chapter_content);
                content.push_str("\n\n---\n\n");
            }
            ExportFormat::Epub => {
                // EPUB handled separately
            }
        }
    }

    let file_name = format!("{}.{}", sanitize_title(title), format.extension());
    let output_path = book_dir.join(&file_name);

    match format {
        ExportFormat::Epub => {
            crate::infrastructure::file_storage::epub::export_epub(book_dir, &output_path, title, "").await?;
        }
        _ => {
            std::fs::write(&output_path, &content)
                .map_err(|e| AppError::internal(format!("Failed to write export: {}", e)))?;
        }
    }

    Ok(ExportArtifact {
        output_path: output_path.to_string_lossy().to_string(),
        file_name,
        chapters_exported: chapter_files.len() as u32,
        total_words,
        format,
    })
}

fn sanitize_title(title: &str) -> String {
    title.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>()
        .chars()
        .take(50)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format() {
        assert_eq!(ExportFormat::parse("txt"), Some(ExportFormat::Txt));
        assert_eq!(ExportFormat::parse("md"), Some(ExportFormat::Md));
        assert_eq!(ExportFormat::parse("epub"), Some(ExportFormat::Epub));
        assert_eq!(ExportFormat::parse("invalid"), None);
        assert_eq!(ExportFormat::Txt.extension(), "txt");
    }

    #[test]
    fn test_sanitize_title() {
        assert_eq!(sanitize_title("Test Book"), "Test_Book");
        // Chinese characters pass is_alphanumeric() in Rust
        assert_eq!(sanitize_title("测试书名"), "测试书名");
        assert_eq!(sanitize_title("test/file"), "test_file");
        assert_eq!(sanitize_title(&"a".repeat(60)), "a".repeat(50));
    }
}
