//! ═══════════════════════════════════════════════════════════════════════════
//! 容错解析 - 配置值容错解析纯函数
//! ═══════════════════════════════════════════════════════════════════════════

/// 容错布尔值解析:容忍 "true"/"1"/"yes"/"on"(大小写不敏感)等常见形式。
pub fn coerce_bool(value: &serde_json::Value, default: bool) -> bool {
    match value {
        serde_json::Value::Bool(b) => *b,
        serde_json::Value::String(s) => {
            let lower = s.to_lowercase();
            match lower.as_str() {
                "true" | "1" | "yes" | "on" => true,
                "false" | "0" | "no" | "off" => false,
                _ => {
                    tracing::warn!(value = %s, default, "无法解析布尔值，使用默认值");
                    default
                }
            }
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return i != 0;
            }
            tracing::warn!(value = %n, default, "无法解析布尔值，使用默认值");
            default
        }
        _ => {
            tracing::warn!(value = %value, default, "无法解析布尔值，使用默认值");
            default
        }
    }
}

/// 容错浮点数解析:接受 Number 或数字字符串,异常时返回默认值。
pub fn coerce_float(value: &serde_json::Value, default: f64) -> f64 {
    match value {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or_else(|| {
            tracing::warn!(value = %n, default, "无法解析浮点数，使用默认值");
            default
        }),
        serde_json::Value::String(s) => s.parse().unwrap_or_else(|_| {
            tracing::warn!(value = %s, default, "无法解析浮点数，使用默认值");
            default
        }),
        _ => {
            tracing::warn!(value = %value, default, "无法解析浮点数，使用默认值");
            default
        }
    }
}

/// 容错整数解析:接受 Number(含浮点截断)或数字字符串,异常时返回默认值。
pub fn coerce_int(value: &serde_json::Value, default: i64) -> i64 {
    match value {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or_else(|| {
            // 尝试浮点转整
            if let Some(f) = n.as_f64() {
                return f as i64;
            }
            tracing::warn!(value = %n, default, "无法解析整数，使用默认值");
            default
        }),
        serde_json::Value::String(s) => s.parse().unwrap_or_else(|_| {
            tracing::warn!(value = %s, default, "无法解析整数，使用默认值");
            default
        }),
        _ => {
            tracing::warn!(value = %value, default, "无法解析整数，使用默认值");
            default
        }
    }
}

/// 容错可选正整数:0/null/负数 disable,异常 warn 后忽略(返回 None)。
pub fn coerce_optional_positive_int(value: &serde_json::Value) -> Option<u32> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i <= 0 {
                    return None;
                }
                return Some(i as u32);
            }
            tracing::warn!(value = %n, "无法解析正整数，忽略");
            None
        }
        serde_json::Value::String(s) => match s.parse::<i64>() {
            Ok(i) if i > 0 => Some(i as u32),
            Ok(_) => None,
            Err(_) => {
                tracing::warn!(value = %s, "无法解析正整数，忽略");
                None
            }
        },
        _ => {
            tracing::warn!(value = %value, "无法解析正整数，忽略");
            None
        }
    }
}

// ── 单元测试 ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_coerce_bool_various() {
        // 原生布尔
        assert_eq!(coerce_bool(&json!(true), false), true);
        assert_eq!(coerce_bool(&json!(false), true), false);

        // 字符串形式(大小写不敏感)
        assert_eq!(coerce_bool(&json!("true"), false), true);
        assert_eq!(coerce_bool(&json!("TRUE"), false), true);
        assert_eq!(coerce_bool(&json!("Yes"), false), true);
        assert_eq!(coerce_bool(&json!("on"), false), true);
        assert_eq!(coerce_bool(&json!("1"), false), true);
        assert_eq!(coerce_bool(&json!("false"), true), false);
        assert_eq!(coerce_bool(&json!("OFF"), true), false);
        assert_eq!(coerce_bool(&json!("0"), true), false);
        assert_eq!(coerce_bool(&json!("no"), true), false);

        // 数字形式
        assert_eq!(coerce_bool(&json!(1), false), true);
        assert_eq!(coerce_bool(&json!(0), true), false);
        assert_eq!(coerce_bool(&json!(-3), false), true);

        // 异常形式 -> 默认值
        assert_eq!(coerce_bool(&json!("maybe"), true), true);
        assert_eq!(coerce_bool(&json!("maybe"), false), false);
        assert_eq!(coerce_bool(&json!("2"), true), true); // 非法字符串 -> default
        assert_eq!(coerce_bool(&json!(null), true), true);
        assert_eq!(coerce_bool(&json!([1, 2]), false), false);
        assert_eq!(coerce_bool(&json!({"a": 1}), true), true);
        // 浮点数字(非整数) -> 默认值
        assert_eq!(coerce_bool(&json!(3.14), true), true);
    }

    #[test]
    fn test_coerce_float_various() {
        // 数字
        assert_eq!(coerce_float(&json!(3), 0.0), 3.0);
        assert_eq!(coerce_float(&json!(3.14), 0.0), 3.14);
        assert_eq!(coerce_float(&json!(-1.5), 0.0), -1.5);
        assert_eq!(coerce_float(&json!(0), 9.9), 0.0);

        // 数字字符串
        assert_eq!(coerce_float(&json!("2.5"), 0.0), 2.5);
        assert_eq!(coerce_float(&json!("-4"), 0.0), -4.0);
        assert_eq!(coerce_float(&json!("1e2"), 0.0), 100.0);

        // 异常 -> 默认值
        assert_eq!(coerce_float(&json!("abc"), 7.7), 7.7);
        assert_eq!(coerce_float(&json!(null), 1.1), 1.1);
        assert_eq!(coerce_float(&json!(true), 2.2), 2.2);
        assert_eq!(coerce_float(&json!([1.0]), 3.3), 3.3);
    }

    #[test]
    fn test_coerce_int_various() {
        // 整数
        assert_eq!(coerce_int(&json!(42), 0), 42);
        assert_eq!(coerce_int(&json!(-5), 0), -5);
        assert_eq!(coerce_int(&json!(0), 9), 0);

        // 浮点截断
        assert_eq!(coerce_int(&json!(3.9), 0), 3);
        assert_eq!(coerce_int(&json!(-2.7), 0), -2);

        // 数字字符串
        assert_eq!(coerce_int(&json!("128"), 0), 128);
        assert_eq!(coerce_int(&json!("-9"), 0), -9);

        // 异常 -> 默认值
        assert_eq!(coerce_int(&json!("abc"), 11), 11);
        assert_eq!(coerce_int(&json!(null), 22), 22);
        assert_eq!(coerce_int(&json!(true), 33), 33);
        assert_eq!(coerce_int(&json!([1]), 44), 44);
    }

    #[test]
    fn test_coerce_optional_positive_int() {
        // null / 0 / 负数 -> None(disable)
        assert_eq!(coerce_optional_positive_int(&json!(null)), None);
        assert_eq!(coerce_optional_positive_int(&json!(0)), None);
        assert_eq!(coerce_optional_positive_int(&json!(-1)), None);
        assert_eq!(coerce_optional_positive_int(&json!(-100)), None);

        // 正整数 -> Some
        assert_eq!(coerce_optional_positive_int(&json!(1)), Some(1));
        assert_eq!(coerce_optional_positive_int(&json!(42)), Some(42));
        assert_eq!(coerce_optional_positive_int(&json!(300)), Some(300));

        // 字符串形式
        assert_eq!(coerce_optional_positive_int(&json!("5")), Some(5));
        assert_eq!(coerce_optional_positive_int(&json!("0")), None);
        assert_eq!(coerce_optional_positive_int(&json!("-3")), None);
        assert_eq!(coerce_optional_positive_int(&json!("abc")), None);

        // 浮点数(非整数) -> warn + None
        assert_eq!(coerce_optional_positive_int(&json!(3.5)), None);

        // 其他类型 -> None
        assert_eq!(coerce_optional_positive_int(&json!(true)), None);
        assert_eq!(coerce_optional_positive_int(&json!([1])), None);
        assert_eq!(coerce_optional_positive_int(&json!({"a": 1})), None);
    }
}