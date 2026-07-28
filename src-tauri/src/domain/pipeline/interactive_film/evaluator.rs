//! ═══════════════════════════════════════════════════════════════════════════
//! Interactive Film Evaluator - 运行时求值
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! - init_var_state：用 variables 的 default 值初始化运行时变量状态
//! - evaluate_condition：根据 op 求值条件（数值比较 / 值相等）
//! - apply_effects：根据 op 修改变量状态（set/add/sub）
//! - visible_choices：过滤掉条件不满足的选项

use std::collections::HashMap;

use serde_json::Value;

use super::graph_schema::{Choice, Condition, ConditionOp, Effect, EffectOp, StoryGraph, StoryNode};

/// 变量状态（运行时）
pub type VarState = HashMap<String, Value>;

/// 初始化变量状态：遍历 graph.variables，用 default 值初始化。
pub fn init_var_state(graph: &StoryGraph) -> VarState {
    let mut state = VarState::new();
    for v in &graph.variables {
        state.insert(v.name.clone(), v.default.clone());
    }
    state
}

/// 求值条件。condition 为 None 时返回 true。
pub fn evaluate_condition(condition: &Condition, state: &VarState) -> bool {
    let lhs = state.get(&condition.var);
    match &condition.op {
        ConditionOp::Eq => lhs == Some(&condition.value),
        ConditionOp::Neq => lhs != Some(&condition.value),
        ConditionOp::Gte | ConditionOp::Lte | ConditionOp::Gt | ConditionOp::Lt => {
            let lhs_num = lhs.and_then(|v| v.as_f64());
            let rhs_num = condition.value.as_f64();
            match (lhs_num, rhs_num) {
                (Some(l), Some(r)) => match &condition.op {
                    ConditionOp::Gte => l >= r,
                    ConditionOp::Lte => l <= r,
                    ConditionOp::Gt => l > r,
                    ConditionOp::Lt => l < r,
                    _ => unreachable!(),
                },
                // 非数值或缺失时，数值比较按 Number(undefined) 语义处理为不成立。
                // Number(undefined) === NaN，任何 NaN 比较均为 false。
                _ => false,
            }
        }
    }
}

/// 应用效果到变量状态（in-place 修改）。
pub fn apply_effects(effects: &[Effect], state: &mut VarState) {
    for e in effects {
        match e.op {
            EffectOp::Set => {
                state.insert(e.var.clone(), e.value.clone());
            }
            EffectOp::Add | EffectOp::Sub => {
                let cur = state.get(&e.var).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let delta = e.value.as_f64().unwrap_or(0.0);
                let next = if e.op == EffectOp::Add {
                    cur + delta
                } else {
                    cur - delta
                };
                // add/sub 结果为 number；保持整数精度时使用整数，否则浮点。
                let next_val = if next.fract() == 0.0 && next.is_finite() {
                    Value::from(next as i64)
                } else {
                    Value::from(next)
                };
                state.insert(e.var.clone(), next_val);
            }
        }
    }
}

/// 可见选项（过滤掉条件不满足的；condition 为 None 视为可见）。
pub fn visible_choices<'a>(node: &'a StoryNode, state: &VarState) -> Vec<&'a Choice> {
    node.choices
        .iter()
        .filter(|c| match &c.condition {
            Some(cond) => evaluate_condition(cond, state),
            None => true,
        })
        .collect()
}
