use crate::errors::AppError;

/// Edit controller for managing truth file edits
pub struct EditController;

impl EditController {
    /// Execute an edit transaction on a truth file
    pub fn execute_edit(
        file_path: &str,
        old_value: &str,
        new_value: &str,
    ) -> Result<EditResult, AppError> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| AppError::internal(format!("Failed to read file: {}", e)))?;

        if !content.contains(old_value) {
            return Err(AppError::bad_request(format!(
                "Old value not found in {}: '{}'",
                file_path, &old_value[..50.min(old_value.len())]
            )));
        }

        let new_content = content.replacen(old_value, new_value, 1);
        std::fs::write(file_path, &new_content)
            .map_err(|e| AppError::internal(format!("Failed to write file: {}", e)))?;

        Ok(EditResult {
            file_path: file_path.to_string(),
            changes: 1,
            old_value: old_value.to_string(),
            new_value: new_value.to_string(),
        })
    }
}

pub struct EditResult {
    pub file_path: String,
    pub changes: u32,
    pub old_value: String,
    pub new_value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_edit_nonexistent_file() {
        let result = EditController::execute_edit("/nonexistent/file.txt", "old", "new");
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_value_not_found() {
        let dir = std::env::temp_dir().join("inkos_test_edit");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test.txt");
        fs::write(&path, "hello world").unwrap();
        let result = EditController::execute_edit(path.to_str().unwrap(), "nonexistent", "new");
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
