// Loop-Engineering 内置模式 —— 4 个 builtin pattern 的声明式定义。
//
// 设计参考:core/agent/loop_engine/types.rs (LoopPatternId 枚举 + BUILTIN_PATTERNS)
// - ChapterWriteLoop:章节写作循环(Plan→Compose→Write)
// - AuditReviseLoop:审计-修订循环(Audit→Revise→Re-audit,带 attempt cap)
// - ObservationLoop:事实观察循环(observer 提取事实 → memory)
// - ConsolidationLoop:章节归档循环(consolidator 压缩历史章节)
//
// 与 core/agent/loop_engine/types.rs::LoopPattern 的关系:
// - core 层的 LoopPattern 是运行时预算配置(用于 check_budget)
// - 本模块的 LoopPatternDto 是前端展示用的完整模式定义(含 phases / cost_config)
// - 二者通过 pattern_id(LoopPatternId::as_str())关联

use crate::infrastructure::db::stores::loop_pattern::LoopPatternRow;

use super::types::{CostConfigDto, PhaseDefDto};

/// 生成 4 个 builtin pattern 的 LoopPatternRow(用于初始化 DB)。
///
/// id 对应 core::agent::loop_engine::types::LoopPatternId::as_str():
/// - "chapter-write-loop"
/// - "audit-revise-loop"
/// - "observation-loop"
/// - "consolidation-loop"
pub fn builtin_pattern_rows() -> Vec<LoopPatternRow> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        LoopPatternRow {
            id: "chapter-write-loop".to_string(),
            name: "Chapter Write Loop".to_string(),
            description: Some("生成下一章正文:Plan → Compose → Write,带字数归一化".to_string()),
            goal: Some("生成下一章正文:Plan → Compose → Write,带字数归一化".to_string()),
            cadence: "per-chapter".to_string(),
            risk_level: "low".to_string(),
            phases: Some(serde_json::to_string(&vec![
                PhaseDefDto { name: "Plan".to_string(), description: "生成章节大纲".to_string(), phase_type: "discover".to_string() },
                PhaseDefDto { name: "Compose".to_string(), description: "组装上下文".to_string(), phase_type: "deliver".to_string() },
                PhaseDefDto { name: "Write".to_string(), description: "生成正文".to_string(), phase_type: "deliver".to_string() },
            ]).unwrap_or_default()),
            human_gates: Some("[]".to_string()),
            cost_config: Some(serde_json::to_string(&CostConfigDto {
                tokens_noop: 5_000,
                tokens_report: 50_000,
                tokens_action: 200_000,
                daily_cap: 1_000_000,
                early_exit_required: false,
            }).unwrap_or_default()),
            skills_required: Some(serde_json::to_string(&vec!["planner".to_string(), "composer".to_string(), "writer".to_string()]).unwrap_or_default()),
            is_active: 1,
            is_builtin: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        LoopPatternRow {
            id: "audit-revise-loop".to_string(),
            name: "Audit Revise Loop".to_string(),
            description: Some("审计-修订循环:Audit → Revise → Re-audit,带 attempt cap 和最佳快照回退".to_string()),
            goal: Some("审计-修订循环:Audit → Revise → Re-audit,带 attempt cap 和最佳快照回退".to_string()),
            cadence: "per-chapter".to_string(),
            risk_level: "high".to_string(),
            phases: Some(serde_json::to_string(&vec![
                PhaseDefDto { name: "Audit".to_string(), description: "审计草稿质量".to_string(), phase_type: "verify".to_string() },
                PhaseDefDto { name: "Revise".to_string(), description: "修复审计发现的问题".to_string(), phase_type: "deliver".to_string() },
                PhaseDefDto { name: "Re-audit".to_string(), description: "重新审计修订后的内容".to_string(), phase_type: "verify".to_string() },
            ]).unwrap_or_default()),
            human_gates: Some(serde_json::to_string(&vec!["max-attempts:3".to_string()]).unwrap_or_default()),
            cost_config: Some(serde_json::to_string(&CostConfigDto {
                tokens_noop: 5_000,
                tokens_report: 50_000,
                tokens_action: 300_000,
                daily_cap: 1_500_000,
                early_exit_required: true,
            }).unwrap_or_default()),
            skills_required: Some(serde_json::to_string(&vec!["auditor".to_string(), "reviser".to_string(), "loop-verifier".to_string(), "minimal-fix".to_string()]).unwrap_or_default()),
            is_active: 1,
            is_builtin: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        LoopPatternRow {
            id: "observation-loop".to_string(),
            name: "Observation Loop".to_string(),
            description: Some("事实观察:从章节提取角色/情节/设定事实,写入 memory".to_string()),
            goal: Some("事实观察:从章节提取角色/情节/设定事实,写入 memory".to_string()),
            cadence: "daily".to_string(),
            risk_level: "low".to_string(),
            phases: Some(serde_json::to_string(&vec![
                PhaseDefDto { name: "Observe".to_string(), description: "从章节提取事实".to_string(), phase_type: "discover".to_string() },
                PhaseDefDto { name: "Persist".to_string(), description: "写入 memory".to_string(), phase_type: "persist".to_string() },
            ]).unwrap_or_default()),
            human_gates: Some("[]".to_string()),
            cost_config: Some(serde_json::to_string(&CostConfigDto {
                tokens_noop: 3_000,
                tokens_report: 20_000,
                tokens_action: 60_000,
                daily_cap: 300_000,
                early_exit_required: false,
            }).unwrap_or_default()),
            skills_required: Some(serde_json::to_string(&vec!["observer".to_string(), "loop-triage".to_string()]).unwrap_or_default()),
            is_active: 1,
            is_builtin: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        LoopPatternRow {
            id: "consolidation-loop".to_string(),
            name: "Consolidation Loop".to_string(),
            description: Some("章节归档:压缩历史章节摘要,释放 context 窗口".to_string()),
            goal: Some("章节归档:压缩历史章节摘要,释放 context 窗口".to_string()),
            cadence: "daily".to_string(),
            risk_level: "medium".to_string(),
            phases: Some(serde_json::to_string(&vec![
                PhaseDefDto { name: "Consolidate".to_string(), description: "压缩历史章节摘要".to_string(), phase_type: "persist".to_string() },
            ]).unwrap_or_default()),
            human_gates: Some("[]".to_string()),
            cost_config: Some(serde_json::to_string(&CostConfigDto {
                tokens_noop: 3_000,
                tokens_report: 30_000,
                tokens_action: 80_000,
                daily_cap: 400_000,
                early_exit_required: false,
            }).unwrap_or_default()),
            skills_required: Some(serde_json::to_string(&vec!["consolidator".to_string()]).unwrap_or_default()),
            is_active: 1,
            is_builtin: 1,
            created_at: now.clone(),
            updated_at: now,
        },
    ]
}

/// 将 builtin patterns 写入 DB(仅当不存在时)。
///
/// 在应用启动时(application/init.rs)调用,确保 builtin patterns 可用。
pub fn ensure_builtin_patterns(db: &crate::infrastructure::db::connection::Database) -> Result<(), crate::shared::error::AppError> {
    for row in builtin_pattern_rows() {
        let existing = db.get_loop_pattern(&row.id)?;
        if existing.is_none() {
            db.upsert_loop_pattern(&row)?;
            tracing::info!(pattern_id = %row.id, "Builtin loop pattern seeded");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_patterns_cover_all_four() {
        let rows = builtin_pattern_rows();
        assert_eq!(rows.len(), 4);
        let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"chapter-write-loop"));
        assert!(ids.contains(&"audit-revise-loop"));
        assert!(ids.contains(&"observation-loop"));
        assert!(ids.contains(&"consolidation-loop"));
    }

    #[test]
    fn builtin_patterns_are_marked_builtin() {
        let rows = builtin_pattern_rows();
        assert!(rows.iter().all(|r| r.is_builtin == 1));
    }

    #[test]
    fn builtin_patterns_have_valid_cost_config() {
        let rows = builtin_pattern_rows();
        for row in &rows {
            let cost_json = row.cost_config.as_ref().expect("cost_config should be set");
            let cost: CostConfigDto = serde_json::from_str(cost_json).expect("cost_config should be valid JSON");
            assert!(cost.daily_cap > 0);
            assert!(cost.tokens_action > 0);
        }
    }

    #[test]
    fn ensure_builtin_patterns_idempotent() {
        let db = crate::infrastructure::db::connection::Database::connect_in_memory().unwrap();
        ensure_builtin_patterns(&db).unwrap();
        ensure_builtin_patterns(&db).unwrap(); // 第二次不应报错

        let patterns = db.list_loop_patterns().unwrap();
        assert_eq!(patterns.len(), 4);
    }
}
