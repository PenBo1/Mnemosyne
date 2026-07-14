// 把 StoryGraph 导出为 Ink 脚本语言。
//
// 映射：
// - VAR 声明变量（flag/bool → 0/1 数值化）
// - === knot === 定义节点
// - * [text] 定义选项，条件用 {cond}，效果用 ~ var op value
// - -> target 跳转，-> END 结局

use crate::shared::error::AppError;

use super::graph_schema::{
    Choice, Condition, ConditionOp, Effect, EffectOp, EndingType, NodeType, StoryGraph,
    StoryNode, VariableType,
};

pub fn export_ink(graph: &StoryGraph) -> Result<String, AppError> {
    let mut out = String::new();
    out.push_str(&format!("// StoryGraph export: {}\n", graph.title));
    out.push_str("// 由 Mnemosyne 生成\n\n");

    // 变量声明
    for v in &graph.variables {
        out.push_str(&format!("VAR {} = {}\n", v.name, ink_var_value(&v.default, &v.var_type)));
    }
    if !graph.variables.is_empty() {
        out.push('\n');
    }

    // 节点 → knot
    for node in &graph.nodes {
        out.push_str(&format!("=== {} ===\n", knot_name(node)));
        if !node.title.is_empty() && node.title != node.id {
            out.push_str(&format!("// {}\n", node.title));
        }
        if !node.scene_desc.is_empty() {
            out.push_str(&node.scene_desc);
            out.push('\n');
        }
        for line in &node.dialogue {
            out.push_str(&format!("{}: {}\n", line.speaker, line.text));
        }

        if node.node_type == NodeType::Ending {
            out.push_str("-> END\n\n");
            continue;
        }

        if node.choices.is_empty() {
            out.push_str("-> END\n\n");
            continue;
        }

        for choice in &node.choices {
            out.push_str(&render_choice(choice));
        }
        out.push('\n');
    }

    // 结局列表注释（供参考）
    if !graph.endings.is_empty() {
        out.push_str("// === Endings ===\n");
        for e in &graph.endings {
            out.push_str(&format!(
                "// {} ({}): {} -> {}\n",
                e.id,
                ending_type_str(&e.ending_type),
                e.title,
                e.node_id
            ));
        }
    }

    Ok(out)
}

fn knot_name(node: &StoryNode) -> &str {
    if node.id.is_empty() {
        "unnamed"
    } else {
        &node.id
    }
}

fn ink_var_value(default: &serde_json::Value, var_type: &VariableType) -> String {
    match var_type {
        VariableType::Flag => {
            if default.as_bool() == Some(true) {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }
        _ => match default {
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => {
                if *b {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            _ => "0".to_string(),
        },
    }
}

fn render_choice(choice: &Choice) -> String {
    let mut line = String::from("* ");
    if let Some(cond) = &choice.condition {
        line.push_str(&format!("{{ {} }} ", render_condition(cond)));
    }
    line.push('[');
    line.push_str(&choice.text);
    line.push_str("] ");
    // 效果
    for effect in &choice.effects {
        line.push('\n');
        line.push_str(&format!("    {}", render_effect(effect)));
    }
    // 跳转
    if !choice.effects.is_empty() {
        line.push('\n');
        line.push_str(&format!("    -> {}", choice.target_node_id));
    } else {
        line.push_str(&format!("-> {}", choice.target_node_id));
    }
    line.push('\n');
    line
}

fn render_condition(cond: &Condition) -> String {
    format!(
        "{} {} {}",
        cond.var,
        op_str(&cond.op),
        ink_literal(&cond.value)
    )
}

fn render_effect(effect: &Effect) -> String {
    match effect.op {
        EffectOp::Set => format!("~ {} = {}", effect.var, ink_literal(&effect.value)),
        EffectOp::Add => format!("~ {} += {}", effect.var, ink_literal(&effect.value)),
        EffectOp::Sub => format!("~ {} -= {}", effect.var, ink_literal(&effect.value)),
    }
}

fn op_str(op: &ConditionOp) -> &'static str {
    match op {
        ConditionOp::Gte => ">=",
        ConditionOp::Lte => "<=",
        ConditionOp::Gt => ">",
        ConditionOp::Lt => "<",
        ConditionOp::Eq => "==",
        ConditionOp::Neq => "!=",
    }
}

fn ink_literal(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Bool(b) => {
            if *b {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}

fn ending_type_str(t: &EndingType) -> &'static str {
    match t {
        EndingType::Good => "good",
        EndingType::Bad => "bad",
        EndingType::Neutral => "neutral",
        EndingType::Secret => "secret",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::pipeline::interactive_film::graph_schema::Variable;

    fn sample_graph() -> StoryGraph {
        StoryGraph {
            schema_version: 1,
            project_id: "p".into(),
            title: "测试".into(),
            world_anchor: None,
            characters: vec![],
            variables: vec![Variable {
                name: "trust".into(),
                var_type: VariableType::Counter,
                default: serde_json::json!(0),
                desc: None,
            }],
            nodes: vec![
                StoryNode {
                    id: "start".into(),
                    title: "开始".into(),
                    node_type: NodeType::Start,
                    scene_desc: "你站在路口".into(),
                    dialogue: vec![],
                    choices: vec![
                        Choice {
                            id: "c1".into(),
                            text: "向左".into(),
                            target_node_id: "end".into(),
                            condition: Some(Condition {
                                var: "trust".into(),
                                op: ConditionOp::Gte,
                                value: serde_json::json!(1),
                            }),
                            effects: vec![Effect {
                                var: "trust".into(),
                                op: EffectOp::Add,
                                value: serde_json::json!(1),
                            }],
                            weight: None,
                        },
                    ],
                    image_slot: None,
                    act: "".into(),
                    position: None,
                },
                StoryNode {
                    id: "end".into(),
                    title: "结局".into(),
                    node_type: NodeType::Ending,
                    scene_desc: "".into(),
                    dialogue: vec![],
                    choices: vec![],
                    image_slot: None,
                    act: "".into(),
                    position: None,
                },
            ],
            endings: vec![],
        }
    }

    #[test]
    fn exports_var_and_knots() {
        let g = sample_graph();
        let ink = export_ink(&g).unwrap();
        assert!(ink.contains("VAR trust = 0"));
        assert!(ink.contains("=== start ==="));
        assert!(ink.contains("=== end ==="));
        assert!(ink.contains("* { trust >= 1 } [向左]"));
        assert!(ink.contains("~ trust += 1"));
        assert!(ink.contains("-> end"));
        assert!(ink.contains("-> END"));
    }
}
