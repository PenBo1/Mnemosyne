use uuid::Uuid;
use crate::shared::error::AppError;

pub fn validate_uuid(id: &str, name: &str) -> Result<Uuid, AppError> {
    if id.is_empty() {
        return Err(AppError::missing_field(name));
    }

    let uuid = Uuid::parse_str(id).map_err(|_| {
        AppError::invalid_format(format!("{} is not a valid UUID: {}", name, id))
    })?;

    Ok(uuid)
}

pub fn validate_uuid_v4(id: &str, name: &str) -> Result<Uuid, AppError> {
    let uuid = validate_uuid(id, name)?;

    if uuid.get_version() != Some(uuid::Version::Random) {
        return Err(AppError::invalid_format(format!(
            "{} must be a UUID v4 (random): {}",
            name, id
        )));
    }

    Ok(uuid)
}

pub fn validate_uuid_v7(id: &str, name: &str) -> Result<Uuid, AppError> {
    let uuid = validate_uuid(id, name)?;

    if uuid.get_version() != Some(uuid::Version::SortRand) {
        return Err(AppError::invalid_format(format!(
            "{} must be a UUID v7 (time-ordered): {}",
            name, id
        )));
    }

    Ok(uuid)
}

pub fn is_valid_uuid(id: &str) -> bool {
    Uuid::parse_str(id).is_ok()
}

pub fn generate_uuid_v4() -> Uuid {
    Uuid::new_v4()
}

pub fn generate_uuid_v7() -> Uuid {
    Uuid::now_v7()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_uuid_valid() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        assert!(validate_uuid(id, "test_id").is_ok());
    }

    #[test]
    fn test_validate_uuid_empty() {
        let result = validate_uuid("", "test_id");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "MISSING_FIELD");
    }

    #[test]
    fn test_validate_uuid_invalid_format() {
        let result = validate_uuid("not-a-uuid", "test_id");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "INVALID_FORMAT");
    }

    #[test]
    fn test_validate_uuid_v4() {
        let v4_id = "550e8400-e29b-41d4-a716-446655440000";
        assert!(validate_uuid_v4(v4_id, "test_id").is_ok());
    }

    #[test]
    fn test_validate_uuid_v4_wrong_version() {
        let v1_id = "550e8400-e29b-11d4-a716-446655440000";
        let result = validate_uuid_v4(v1_id, "test_id");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_valid_uuid() {
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_valid_uuid("not-a-uuid"));
        assert!(!is_valid_uuid(""));
    }
}