//! ═══════════════════════════════════════════════════════════════════════════
//! validator - 参数验证模块
//! ═══════════════════════════════════════════════════════════════════════════

use std::collections::HashMap;

use serde_json::Value;

use crate::shared::error::AppError;

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct ParameterSchema {
    pub required_fields: Vec<String>,
    pub field_types: HashMap<String, FieldType>,
    pub nested_schemas: HashMap<String, Box<ParameterSchema>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    String,
    Number,
    Boolean,
    Object,
    Array,
    Null,
    Any,
}


impl ParameterSchema {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn require(mut self, field: impl Into<String>) -> Self {
        self.required_fields.push(field.into());
        self
    }

    pub fn field_type(mut self, field: impl Into<String>, ty: FieldType) -> Self {
        self.field_types.insert(field.into(), ty);
        self
    }

    pub fn nested(mut self, field: impl Into<String>, schema: ParameterSchema) -> Self {
        self.nested_schemas.insert(field.into(), Box::new(schema));
        self
    }
}

pub struct ParameterValidator {
    strict_mode: bool,
}

impl ParameterValidator {
    pub fn new() -> Self {
        Self { strict_mode: false }
    }

    pub fn strict() -> Self {
        Self { strict_mode: true }
    }

    pub fn validate_shape(&self, params: &Value, schema: &ParameterSchema) -> Result<(), AppError> {
        let obj = params.as_object().ok_or_else(|| {
            AppError::invalid_input("Parameter validation failed: expected object")
        })?;

        self.validate_required_fields(obj, schema)?;
        self.validate_types(obj, schema)?;

        Ok(())
    }

    pub fn validate_required_fields(
        &self,
        obj: &serde_json::Map<String, Value>,
        schema: &ParameterSchema,
    ) -> Result<(), AppError> {
        for field in &schema.required_fields {
            if !obj.contains_key(field) {
                return Err(AppError::missing_field(field.clone()));
            }
        }

        Ok(())
    }

    pub fn validate_types(
        &self,
        obj: &serde_json::Map<String, Value>,
        schema: &ParameterSchema,
    ) -> Result<(), AppError> {
        for (field, expected_type) in &schema.field_types {
            if let Some(value) = obj.get(field) {
                self.validate_field_type(field, value, expected_type)?;

                if let Some(nested_schema) = schema.nested_schemas.get(field) {
                    self.validate_nested_schema(field, value, nested_schema)?;
                }
            } else if self.strict_mode && schema.required_fields.contains(field) {
                return Err(AppError::missing_field(field.clone()));
            }
        }

        Ok(())
    }

    fn validate_field_type(
        &self,
        field: &str,
        value: &Value,
        expected: &FieldType,
    ) -> Result<(), AppError> {
        let actual_matches = matches!(
            (expected, value),
            (FieldType::String, Value::String(_))
                | (FieldType::Number, Value::Number(_))
                | (FieldType::Boolean, Value::Bool(_))
                | (FieldType::Object, Value::Object(_))
                | (FieldType::Array, Value::Array(_))
                | (FieldType::Null, Value::Null)
                | (FieldType::Any, _)
        );

        if !actual_matches {
            let actual_type = self.value_type_name(value);
            return Err(AppError::invalid_input(format!(
                "Type mismatch for field '{}': expected {:?}, got {}",
                field, expected, actual_type
            )));
        }

        Ok(())
    }

    fn validate_nested_schema(
        &self,
        field: &str,
        value: &Value,
        nested_schema: &ParameterSchema,
    ) -> Result<(), AppError> {
        let nested_obj = value.as_object().ok_or_else(|| {
            AppError::invalid_input(format!(
                "Nested field '{}' must be an object",
                field
            ))
        })?;

        self.validate_required_fields(nested_obj, nested_schema)?;
        self.validate_types(nested_obj, nested_schema)?;

        Ok(())
    }

    fn value_type_name(&self, value: &Value) -> &'static str {
        match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    pub fn validate_json_string(&self, json_str: &str) -> Result<Value, AppError> {
        serde_json::from_str(json_str).map_err(|e| {
            AppError::invalid_format(format!("Invalid JSON: {}", e))
        })
    }

    pub fn validate_object<'a>(
        &self,
        value: &'a Value,
        field_hint: &str,
    ) -> Result<&'a serde_json::Map<String, Value>, AppError> {
        value.as_object().ok_or_else(|| {
            AppError::invalid_input(format!(
                "Field '{}' must be an object",
                field_hint
            ))
        })
    }

    pub fn validate_array<'a>(&self, value: &'a Value, field_hint: &str) -> Result<&'a Vec<Value>, AppError> {
        value.as_array().ok_or_else(|| {
            AppError::invalid_input(format!(
                "Field '{}' must be an array",
                field_hint
            ))
        })
    }

    pub fn validate_string<'a>(&self, value: &'a Value, field_hint: &str) -> Result<&'a str, AppError> {
        value.as_str().ok_or_else(|| {
            AppError::invalid_input(format!(
                "Field '{}' must be a string",
                field_hint
            ))
        })
    }

    pub fn validate_number(&self, value: &Value, field_hint: &str) -> Result<f64, AppError> {
        value.as_f64().ok_or_else(|| {
            AppError::invalid_input(format!(
                "Field '{}' must be a number",
                field_hint
            ))
        })
    }

    pub fn validate_boolean(&self, value: &Value, field_hint: &str) -> Result<bool, AppError> {
        value.as_bool().ok_or_else(|| {
            AppError::invalid_input(format!(
                "Field '{}' must be a boolean",
                field_hint
            ))
        })
    }
}

impl Default for ParameterValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_required_fields_success() {
        let validator = ParameterValidator::new();
        let schema = ParameterSchema::new()
            .require("name")
            .require("version");
        let obj = serde_json::json!({"name": "test", "version": "1.0"});

        let result = validator.validate_shape(&obj, &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_required_fields_missing() {
        let validator = ParameterValidator::new();
        let schema = ParameterSchema::new().require("name").require("version");
        let obj = serde_json::json!({"name": "test"});

        let result = validator.validate_shape(&obj, &schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("version"));
    }

    #[test]
    fn test_validate_types_success() {
        let validator = ParameterValidator::new();
        let schema = ParameterSchema::new()
            .field_type("name", FieldType::String)
            .field_type("count", FieldType::Number);
        let obj = serde_json::json!({"name": "test", "count": 42});

        let result = validator.validate_shape(&obj, &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_types_mismatch() {
        let validator = ParameterValidator::new();
        let schema = ParameterSchema::new()
            .field_type("count", FieldType::Number);
        let obj = serde_json::json!({"count": "not a number"});

        let result = validator.validate_shape(&obj, &schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Type mismatch"));
    }

    #[test]
    fn test_validate_nested_schema() {
        let validator = ParameterValidator::new();
        let nested = ParameterSchema::new()
            .require("id")
            .field_type("id", FieldType::String);
        let schema = ParameterSchema::new()
            .field_type("metadata", FieldType::Object)
            .nested("metadata", nested);
        let obj = serde_json::json!({"metadata": {"id": "123"}});

        let result = validator.validate_shape(&obj, &schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_string() {
        let validator = ParameterValidator::new();

        let valid = validator.validate_json_string("{\"key\": \"value\"}");
        assert!(valid.is_ok());

        let invalid = validator.validate_json_string("not json");
        assert!(invalid.is_err());
    }
}