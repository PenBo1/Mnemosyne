//! ═══════════════════════════════════════════════════════════════════════════
//! ToolRequirement - 工具需求声明式依赖系统
//! ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

// ── ToolKind: 工具类型分类 ──────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Read,
    Write,
    Edit,
    Delete,
    Search,
    Execute,
    List,
    Create,
    Move,
    Copy,
    Network,
    Mcp,
    Custom(String),
}

impl ToolKind {
    pub fn as_str(&self) -> &str {
        match self {
            ToolKind::Read => "read",
            ToolKind::Write => "write",
            ToolKind::Edit => "edit",
            ToolKind::Delete => "delete",
            ToolKind::Search => "search",
            ToolKind::Execute => "execute",
            ToolKind::List => "list",
            ToolKind::Create => "create",
            ToolKind::Move => "move",
            ToolKind::Copy => "copy",
            ToolKind::Network => "network",
            ToolKind::Mcp => "mcp",
            ToolKind::Custom(s) => s.as_str(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "read" => ToolKind::Read,
            "write" => ToolKind::Write,
            "edit" => ToolKind::Edit,
            "delete" => ToolKind::Delete,
            "search" => ToolKind::Search,
            "execute" => ToolKind::Execute,
            "list" => ToolKind::List,
            "create" => ToolKind::Create,
            "move" => ToolKind::Move,
            "copy" => ToolKind::Copy,
            "network" => ToolKind::Network,
            "mcp" => ToolKind::Mcp,
            other => ToolKind::Custom(other.to_string()),
        }
    }
}

impl std::fmt::Display for ToolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── ToolId: 工具标识 ────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolId {
    pub namespace: String,
    pub id: String,
}

impl ToolId {
    pub fn new(namespace: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            id: id.into(),
        }
    }

    pub fn full_name(&self) -> String {
        format!("{}:{}", self.namespace, self.id)
    }
}

impl std::fmt::Display for ToolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.namespace, self.id)
    }
}

// ── Expr<T>: 布尔表达式树 ───────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Expr<T> {
    True,
    False,
    Value(T),
    Not(Box<Expr<T>>),
    And(Vec<Expr<T>>),
    Or(Vec<Expr<T>>),
}

impl<T> Expr<T> {
    pub fn true_() -> Self {
        Expr::True
    }

    pub fn false_() -> Self {
        Expr::False
    }

    pub fn value(v: T) -> Self {
        Expr::Value(v)
    }

    pub fn not(expr: Expr<T>) -> Self {
        Expr::Not(Box::new(expr))
    }

    pub fn and(exprs: Vec<Expr<T>>) -> Self {
        Expr::And(exprs)
    }

    pub fn or(exprs: Vec<Expr<T>>) -> Self {
        Expr::Or(exprs)
    }

    pub fn eval<F>(&self, f: &F) -> bool
    where
        F: Fn(&T) -> bool,
    {
        match self {
            Expr::True => true,
            Expr::False => false,
            Expr::Value(v) => f(v),
            Expr::Not(inner) => !inner.eval(f),
            Expr::And(exprs) => exprs.iter().all(|e| e.eval(f)),
            Expr::Or(exprs) => exprs.iter().any(|e| e.eval(f)),
        }
    }
}

// ── ParamCondition: 参数条件 ─────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", content = "args")]
pub enum ParamCondition {
    Exists(String),
    Equals { key: String, value: JsonValue },
    NotEquals { key: String, value: JsonValue },
    Contains { key: String, value: String },
    GreaterThan { key: String, value: f64 },
    LessThan { key: String, value: f64 },
    Matches { key: String, pattern: String },
}

impl ParamCondition {
    pub fn eval(&self, params: &HashMap<String, JsonValue>) -> bool {
        match self {
            ParamCondition::Exists(key) => params.contains_key(key),
            ParamCondition::Equals { key, value } => params
                .get(key)
                .map(|v| v == value)
                .unwrap_or(false),
            ParamCondition::NotEquals { key, value } => params
                .get(key)
                .map(|v| v != value)
                .unwrap_or(true),
            ParamCondition::Contains { key, value } => params
                .get(key)
                .and_then(|v| v.as_str())
                .map(|s| s.contains(value))
                .unwrap_or(false),
            ParamCondition::GreaterThan { key, value } => params
                .get(key)
                .and_then(|v| v.as_f64())
                .map(|v| v > *value)
                .unwrap_or(false),
            ParamCondition::LessThan { key, value } => params
                .get(key)
                .and_then(|v| v.as_f64())
                .map(|v| v < *value)
                .unwrap_or(false),
            ParamCondition::Matches { key, pattern } => {
                let regex = regex::Regex::new(pattern);
                params
                    .get(key)
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        regex
                            .map(|re| re.is_match(s))
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
            }
        }
    }
}

// ── ToolParamsRequirement: 参数条件检查 ──────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolParamsRequirement {
    pub conditions: Expr<ParamCondition>,
}

impl ToolParamsRequirement {
    pub fn new(conditions: Expr<ParamCondition>) -> Self {
        Self { conditions }
    }

    pub fn eval(&self, params: &HashMap<String, JsonValue>) -> bool {
        self.conditions.eval(&|c| c.eval(params))
    }

    pub fn from_single(condition: ParamCondition) -> Self {
        Self {
            conditions: Expr::value(condition),
        }
    }
}

// ── ToolRequirement: 工具需求声明 ────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum ToolRequirement {
    Tool {
        id: ToolId,
    },
    ToolKind {
        kind: ToolKind,
    },
    IfParams {
        condition: ToolParamsRequirement,
        requirement: Box<ToolRequirement>,
    },
    And(Vec<ToolRequirement>),
    Or(Vec<ToolRequirement>),
    Not(Box<ToolRequirement>),
}

impl ToolRequirement {
    pub fn tool(namespace: impl Into<String>, id: impl Into<String>) -> Self {
        ToolRequirement::Tool {
            id: ToolId::new(namespace, id),
        }
    }

    pub fn tool_kind(kind: ToolKind) -> Self {
        ToolRequirement::ToolKind { kind }
    }

    pub fn if_params(
        condition: ToolParamsRequirement,
        requirement: ToolRequirement,
    ) -> Self {
        ToolRequirement::IfParams {
            condition,
            requirement: Box::new(requirement),
        }
    }

    pub fn and(requirements: Vec<ToolRequirement>) -> Self {
        ToolRequirement::And(requirements)
    }

    pub fn or(requirements: Vec<ToolRequirement>) -> Self {
        ToolRequirement::Or(requirements)
    }

    pub fn not(requirement: ToolRequirement) -> Self {
        ToolRequirement::Not(Box::new(requirement))
    }
}

// ── RequirementChecker: 需求检查器 ───────────────────────────────

pub trait RequirementChecker {
    fn has_tool(&self, id: &ToolId) -> bool;
    fn has_tool_kind(&self, kind: &ToolKind) -> bool;
    fn get_params(&self) -> &HashMap<String, JsonValue>;
}

impl ToolRequirement {
    pub fn eval(&self, checker: &dyn RequirementChecker) -> bool {
        match self {
            ToolRequirement::Tool { id } => checker.has_tool(id),
            ToolRequirement::ToolKind { kind } => checker.has_tool_kind(kind),
            ToolRequirement::IfParams {
                condition,
                requirement,
            } => {
                if condition.eval(checker.get_params()) {
                    requirement.eval(checker)
                } else {
                    true
                }
            }
            ToolRequirement::And(requirements) => {
                requirements.iter().all(|r| r.eval(checker))
            }
            ToolRequirement::Or(requirements) => {
                requirements.iter().any(|r| r.eval(checker))
            }
            ToolRequirement::Not(inner) => !inner.eval(checker),
        }
    }
}

// ── DefaultRequirementChecker: 默认检查器实现 ─────────────────────

#[derive(Debug, Clone, Default)]
pub struct DefaultRequirementChecker {
    pub available_tools: HashMap<ToolId, bool>,
    pub available_kinds: HashMap<ToolKind, bool>,
    pub params: HashMap<String, JsonValue>,
}

impl DefaultRequirementChecker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tool(mut self, id: ToolId) -> Self {
        self.available_tools.insert(id, true);
        self
    }

    pub fn with_kind(mut self, kind: ToolKind) -> Self {
        self.available_kinds.insert(kind, true);
        self
    }

    pub fn with_param(mut self, key: impl Into<String>, value: JsonValue) -> Self {
        self.params.insert(key.into(), value);
        self
    }
}

impl RequirementChecker for DefaultRequirementChecker {
    fn has_tool(&self, id: &ToolId) -> bool {
        self.available_tools.get(id).copied().unwrap_or(false)
    }

    fn has_tool_kind(&self, kind: &ToolKind) -> bool {
        self.available_kinds.get(kind).copied().unwrap_or(false)
    }

    fn get_params(&self) -> &HashMap<String, JsonValue> {
        &self.params
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_kind() {
        assert_eq!(ToolKind::Read.as_str(), "read");
        assert_eq!(ToolKind::from_str("write"), ToolKind::Write);
        assert_eq!(
            ToolKind::from_str("custom_op"),
            ToolKind::Custom("custom_op".to_string())
        );
    }

    #[test]
    fn test_tool_id() {
        let id = ToolId::new("fs", "read_file");
        assert_eq!(id.full_name(), "fs:read_file");
        assert_eq!(format!("{}", id), "fs:read_file");
    }

    #[test]
    fn test_expr_basic() {
        let expr: Expr<bool> = Expr::true_();
        assert!(expr.eval(&|_: &bool| unreachable!()));

        let expr: Expr<bool> = Expr::false_();
        assert!(!expr.eval(&|_: &bool| unreachable!()));

        let expr: Expr<i32> = Expr::value(42);
        assert!(expr.eval(&|v| *v == 42));
        assert!(!expr.eval(&|v| *v == 0));
    }

    #[test]
    fn test_expr_composite() {
        let expr: Expr<i32> = Expr::and(vec![
            Expr::value(1),
            Expr::value(2),
            Expr::value(3),
        ]);
        assert!(expr.eval(&|v| *v > 0));

        let expr: Expr<i32> = Expr::or(vec![
            Expr::value(0),
            Expr::value(1),
        ]);
        assert!(expr.eval(&|v| *v >= 0));

        let expr: Expr<i32> = Expr::not(Expr::value(0));
        assert!(expr.eval(&|v| *v == 0));
    }

    #[test]
    fn test_param_condition() {
        let mut params = HashMap::new();
        params.insert("path".to_string(), json!("/tmp/test.txt"));
        params.insert("size".to_string(), json!(1024));
        params.insert("mode".to_string(), json!("read"));

        assert!(ParamCondition::Exists("path".to_string()).eval(&params));
        assert!(!ParamCondition::Exists("missing".to_string()).eval(&params));

        assert!(ParamCondition::Equals {
            key: "mode".to_string(),
            value: json!("read")
        }
        .eval(&params));

        assert!(ParamCondition::Contains {
            key: "path".to_string(),
            value: "test".to_string()
        }
        .eval(&params));

        assert!(ParamCondition::GreaterThan {
            key: "size".to_string(),
            value: 500.0
        }
        .eval(&params));
    }

    #[test]
    fn test_tool_params_requirement() {
        let mut params = HashMap::new();
        params.insert("recursive".to_string(), json!(true));

        let req = ToolParamsRequirement::from_single(ParamCondition::Equals {
            key: "recursive".to_string(),
            value: json!(true),
        });
        assert!(req.eval(&params));

        let req = ToolParamsRequirement::new(Expr::and(vec![
            Expr::Value(ParamCondition::Equals {
                key: "recursive".to_string(),
                value: json!(true),
            }),
            Expr::Value(ParamCondition::Exists("path".to_string())),
        ]));
        assert!(!req.eval(&params));
    }

    #[test]
    fn test_tool_requirement() {
        let checker = DefaultRequirementChecker::new()
            .with_tool(ToolId::new("fs", "read_file"))
            .with_kind(ToolKind::Read)
            .with_param("mode", json!("read"));

        let req = ToolRequirement::tool("fs", "read_file");
        assert!(req.eval(&checker));

        let req = ToolRequirement::tool_kind(ToolKind::Read);
        assert!(req.eval(&checker));

        let req = ToolRequirement::tool("fs", "write_file");
        assert!(!req.eval(&checker));

        let req = ToolRequirement::and(vec![
            ToolRequirement::tool("fs", "read_file"),
            ToolRequirement::tool_kind(ToolKind::Read),
        ]);
        assert!(req.eval(&checker));

        let req = ToolRequirement::or(vec![
            ToolRequirement::tool("fs", "write_file"),
            ToolRequirement::tool_kind(ToolKind::Read),
        ]);
        assert!(req.eval(&checker));
    }

    #[test]
    fn test_if_params_requirement() {
        let mut checker = DefaultRequirementChecker::new()
            .with_tool(ToolId::new("fs", "write_file"))
            .with_kind(ToolKind::Write);
        checker.params.insert("force".to_string(), json!(true));

        let req = ToolRequirement::if_params(
            ToolParamsRequirement::from_single(ParamCondition::Equals {
                key: "force".to_string(),
                value: json!(true),
            }),
            ToolRequirement::tool("fs", "write_file"),
        );
        assert!(req.eval(&checker));

        checker.params.insert("force".to_string(), json!(false));
        assert!(req.eval(&checker));
    }
}