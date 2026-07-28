//! ContextCompressor 集成测试：4 阶段压缩流程、anti-thrashing、cooldown、focus topic 等。

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::infrastructure::llm::types::Message;

use super::*;
use super::sanitize::sanitize_tool_pairs;
use super::summary::SummaryLlm;
use super::test_support::*;

// ---------- Phase 1: tool output pruning ----------

#[test]
fn phase1_prunes_long_tool_output() {
    let c = build_compressor(MockSummaryLlm::ok("{}"));
    let long_content = "first line of tool output\n".to_string() + &"x".repeat(3000);
    let messages = vec![
        user_msg("please run tool"),
        assistant_with_tool_call("tc1", "search"),
        tool_msg("tc1", &long_content),
    ];

    let pruned = c.phase1_prune_tool_outputs(messages);
    let tool = pruned.iter().find(|m| m.role == "tool").unwrap();
    assert!(tool.content.chars().count() < long_content.chars().count());
    assert!(tool.content.contains("[truncated,"));
    assert!(tool.content.contains("first line of tool output"));
}

#[test]
fn phase1_keeps_short_tool_output() {
    let c = build_compressor(MockSummaryLlm::ok("{}"));
    let short = "small output".to_string();
    let messages = vec![tool_msg("tc1", &short)];
    let pruned = c.phase1_prune_tool_outputs(messages);
    assert_eq!(pruned[0].content, short);
}

// ---------- Phase 2: tail protection ----------

#[test]
fn phase2_tail_protection_keeps_tail() {
    let c = build_compressor(MockSummaryLlm::ok("{}"));
    let messages: Vec<Message> =
        (0..30).map(|i| user_msg(&format!("message {i}"))).collect();
    let (head, middle, tail) = c.phase2_split_protected(&messages);
    assert_eq!(head.len(), 3, "head = protect_first_n");
    assert_eq!(tail.len(), 20, "tail = protect_last_n");
    assert_eq!(middle.len(), 7, "middle = 30 - 3 - 20");
    assert_eq!(head[0].content, "message 0");
    assert_eq!(tail.last().unwrap().content, "message 29");
}

#[test]
fn phase2_floor_8_when_protect_last_n_small() {
    // protect_last_n = 2，应被 floor 到 8。
    let c = ContextCompressor {
        threshold_percent: 0.5,
        protect_first_n: 3,
        protect_last_n: 2,
        summary_target_ratio: 0.2,
        context_length: 10_000,
        llm: MockSummaryLlm::ok("{}"),
        state: Arc::new(RwLock::new(CompressorState::default())),
    };
    let messages: Vec<Message> = (0..20).map(|i| user_msg(&format!("m{i}"))).collect();
    let (_head, _middle, tail) = c.phase2_split_protected(&messages);
    assert_eq!(tail.len(), 8, "floor 8 应保证 tail 至少 8 条");
}

// ---------- Phase 3: LLM summarization ----------

#[tokio::test]
async fn phase3_llm_summary_assembles_structured_json() {
    let summary_json = r#"{
            "active_task": "writing a novel",
            "completed_actions": ["outlined plot", "wrote chapter 1"],
            "key_decisions": ["use third-person POV"],
            "pending_questions": ["what is the protagonist's name?"],
            "relevant_context": "fantasy genre, 80k words target"
        }"#;
    let c = build_compressor(MockSummaryLlm::ok(summary_json));
    let middle = vec![
        user_msg("help me write a novel"),
        assistant_msg("sure, let's outline the plot"),
    ];
    let msg = c
        .phase3_llm_summarize(&middle, Some("novel writing"), None)
        .await
        .unwrap();
    assert_eq!(msg.role, "system");
    assert!(msg.content.starts_with("[CONTEXT_SUMMARY]"));
    assert!(msg.content.contains("[SYSTEM NOTE:"));
    assert!(msg.content.contains("writing a novel"));
    assert!(msg.content.contains("outlined plot"));
}

#[tokio::test]
async fn phase3_degrades_to_unparsed_on_invalid_json() {
    let c = build_compressor(MockSummaryLlm::ok("not valid json"));
    let middle = vec![user_msg("hi")];
    let msg = c.phase3_llm_summarize(&middle, None, None).await.unwrap();
    // 应回退到把原始响应塞入 relevant_context。
    assert!(msg.content.contains("not valid json"));
    assert!(msg.content.contains("(unparsed summary)"));
}

// ---------- Phase 4: static fallback ----------

#[tokio::test]
async fn phase4_static_fallback_on_llm_failure() {
    let c = build_compressor(MockSummaryLlm::err());
    let middle: Vec<Message> = (0..10).map(|i| user_msg(&format!("m{i}"))).collect();
    let msg = c.phase4_static_fallback(&middle);
    assert_eq!(msg.role, "system");
    assert!(msg.content.contains("10 messages compressed"));
    assert!(msg.content.contains("[CONTEXT_SUMMARY]"));
    assert!(msg.content.contains("[SYSTEM NOTE:"));
}

#[tokio::test]
async fn phase4_invoked_when_llm_fails_in_compress() {
    let c = build_compressor(MockSummaryLlm::err());
    let messages: Vec<Message> = (0..30).map(|i| user_msg(&format!("m{i}"))).collect();
    let result = c.compress(messages, 8_000, None).await.unwrap();
    // 应触发 Phase 4。
    let summary_msg = result
        .iter()
        .find(|m| m.content.contains("[CONTEXT_SUMMARY]"))
        .unwrap();
    assert!(summary_msg.content.contains("messages compressed"));
}

// ---------- 整体压缩流程 ----------

#[tokio::test]
async fn compress_reduces_message_count() {
    let summary_json = r#"{"active_task":"t","completed_actions":[],"key_decisions":[],"pending_questions":[],"relevant_context":""}"#;
    let c = build_compressor(MockSummaryLlm::ok(summary_json));
    let messages: Vec<Message> = (0..50).map(|i| user_msg(&format!("m{i}"))).collect();
    let original = messages.len();
    let result = c.compress(messages, 8_000, None).await.unwrap();
    assert!(result.len() < original, "压缩后消息数应小于原始");
    // 3 head + 1 summary + 20 tail = 24。
    assert_eq!(result.len(), 24);
}

#[tokio::test]
async fn compress_skips_when_below_threshold() {
    // LLM 设为失败，确保不应被调用。
    let c = build_compressor(MockSummaryLlm::err());
    let messages = vec![user_msg("hi"), assistant_msg("hello")];
    let result = c.compress(messages, 100, None).await.unwrap();
    // 低于阈值，仅 Phase 1 执行，不压缩。
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn compress_updates_compression_count() {
    let summary_json = r#"{"active_task":"t","completed_actions":[],"key_decisions":[],"pending_questions":[],"relevant_context":""}"#;
    let c = build_compressor(MockSummaryLlm::ok(summary_json));
    let messages: Vec<Message> = (0..30).map(|i| user_msg(&format!("m{i}"))).collect();
    let _ = c.compress(messages, 8_000, None).await.unwrap();
    let status = c.get_status().await;
    assert_eq!(status.compression_count, 1);
    assert_eq!(status.threshold_tokens, 5_000);
    assert_eq!(status.context_length, 10_000);
}

#[tokio::test]
async fn update_from_response_updates_state() {
    let c = build_compressor(MockSummaryLlm::ok("{}"));
    c.update_from_response(100, 50).await.unwrap();
    let status = c.get_status().await;
    assert_eq!(status.last_prompt_tokens, 100);
    assert_eq!(status.last_completion_tokens, 50);
    assert_eq!(status.last_total_tokens, 150);
}

#[tokio::test]
async fn should_compress_threshold_check() {
    let c = build_compressor(MockSummaryLlm::ok("{}"));
    // 阈值 = 10000 * 0.5 = 5000。
    assert!(c.should_compress(5000).await);
    assert!(c.should_compress(6000).await);
    assert!(!c.should_compress(4999).await);
}

// ---------- SubTask 11.1: iterative summary 集成 ----------

#[tokio::test]
async fn iterative_summary_passes_previous_summary_to_llm() {
    let summary_json = r#"{"active_task":"t","completed_actions":[],"key_decisions":[],"pending_questions":[],"relevant_context":""}"#;
    let mock = CapturingSummaryLlm::ok(summary_json);
    // build_compressor 需要 Arc<dyn SummaryLlm>，通过 unsized coercion 转换。
    let c = build_compressor(mock.clone());

    // 构造消息：在中间位置放一条已有的 [CONTEXT_SUMMARY] 消息。
    let prev_summary_content = format!(
        "{}\n{}\n{{\"active_task\":\"old task\"}}",
        SUMMARY_MARKER, SUMMARY_HIJACK_GUARD
    );
    let mut messages: Vec<Message> = Vec::new();
    for i in 0..30 {
        if i == 5 {
            messages.push(Message {
                role: "system".to_string(),
                content: prev_summary_content.clone(),
                tool_calls: None,
                tool_call_id: None,
            });
        } else {
            messages.push(user_msg(&format!("m{i}")));
        }
    }

    let _ = c.compress(messages, 8_000, None).await.unwrap();

    // 验证 LLM 收到的 user 参数包含 "Previous summary:"。
    let captured = mock.captured.lock().unwrap();
    assert!(
        captured
            .iter()
            .any(|(_, user)| user.contains("Previous summary:")),
        "Phase 3 应将 previous_summary 注入 LLM user prompt"
    );
    // 验证内容确实包含旧的 active_task。
    assert!(captured.iter().any(|(_, user)| user.contains("old task")));
}

#[tokio::test]
async fn iterative_summary_omitted_when_no_previous() {
    let summary_json = r#"{"active_task":"t","completed_actions":[],"key_decisions":[],"pending_questions":[],"relevant_context":""}"#;
    let mock = CapturingSummaryLlm::ok(summary_json);
    let c = build_compressor(mock.clone());

    // 无 [CONTEXT_SUMMARY] 消息。
    let messages: Vec<Message> = (0..30).map(|i| user_msg(&format!("m{i}"))).collect();
    let _ = c.compress(messages, 8_000, None).await.unwrap();

    let captured = mock.captured.lock().unwrap();
    assert!(
        captured
            .iter()
            .all(|(_, user)| !user.contains("Previous summary:")),
        "无 previous_summary 时不应注入该前缀"
    );
}

// ---------- SubTask 11.2: anti-thrashing ----------

#[tokio::test]
async fn anti_thrashing_blocks_after_two_ineffective_compressions() {
    let summary_json = r#"{"active_task":"t","completed_actions":[],"key_decisions":[],"pending_questions":[],"relevant_context":""}"#;
    // 阈值降至 10 tokens（threshold_percent=0.001），使 current_tokens=100 超过阈值进入
    // 完整压缩流程；30 条短消息压缩后 token 数仍 > 100，节省比例为负 < 10% → 触发 anti-thrashing。
    let c = ContextCompressor {
        threshold_percent: 0.001,
        protect_first_n: 3,
        protect_last_n: 20,
        summary_target_ratio: 0.2,
        context_length: 10_000,
        llm: MockSummaryLlm::ok(summary_json),
        state: Arc::new(RwLock::new(CompressorState::default())),
    };

    // 构造低效压缩场景：current_tokens 极小（100），但压缩后 token 数相近甚至更多
    // （30 条消息 + summary ≈ 数百 token），导致节省比例为负 < 10%。
    let messages: Vec<Message> = (0..30).map(|i| user_msg(&format!("m{i}"))).collect();

    // 第一次压缩：节省 < 10% → count = 1。
    let _ = c.compress(messages.clone(), 100, None).await.unwrap();
    {
        let s = c.state.read().await;
        assert_eq!(s.ineffective_compression_count, 1, "第一次低效后 count=1");
    }
    // count < 2，should_compress 仍允许。
    assert!(c.should_compress(8_000).await);

    // 第二次压缩：节省 < 10% → count = 2。
    let _ = c.compress(messages, 100, None).await.unwrap();
    {
        let s = c.state.read().await;
        assert_eq!(s.ineffective_compression_count, 2, "第二次低效后 count=2");
    }
    // count >= 2，should_compress 拒绝（即使超过阈值）。
    assert!(
        !c.should_compress(8_000).await,
        "连续 2 次低效后应阻止压缩"
    );
}

#[tokio::test]
async fn anti_thrashing_resets_on_effective_compression() {
    let summary_json = r#"{"active_task":"t","completed_actions":[],"key_decisions":[],"pending_questions":[],"relevant_context":""}"#;
    let c = build_compressor(MockSummaryLlm::ok(summary_json));

    // 手动将 count 设为 1，模拟已发生一次低效压缩。
    {
        let mut s = c.state.write().await;
        s.ineffective_compression_count = 1;
    }
    assert!(c.should_compress(8_000).await);

    // 一次有效压缩（current_tokens=8000，压缩后 token 远小 → 节省 > 10%）应重置 count。
    let messages: Vec<Message> = (0..30).map(|i| user_msg(&format!("m{i}"))).collect();
    let _ = c.compress(messages, 8_000, None).await.unwrap();
    {
        let s = c.state.read().await;
        assert_eq!(s.ineffective_compression_count, 0, "有效压缩应重置 count");
    }
    assert!(c.should_compress(8_000).await);
}

#[tokio::test]
async fn anti_thrashing_resets_on_update_from_response() {
    let c = build_compressor(MockSummaryLlm::ok("{}"));
    // 手动将 count 设为 2，模拟触发 anti-thrashing。
    {
        let mut s = c.state.write().await;
        s.ineffective_compression_count = 2;
    }
    assert!(!c.should_compress(8_000).await);

    // 新 API 响应到来 → 重置 anti-thrashing 计数。
    c.update_from_response(1000, 500).await.unwrap();
    assert!(
        c.should_compress(8_000).await,
        "update_from_response 应重置 anti-thrashing 计数"
    );
}

// ---------- SubTask 11.3: cooldown ----------

#[tokio::test]
async fn cooldown_blocks_should_compress_after_failure() {
    let c = build_compressor(MockSummaryLlm::err());
    let messages: Vec<Message> = (0..30).map(|i| user_msg(&format!("m{i}"))).collect();
    // 触发 Phase 3 失败 → 设置 cooldown（mock 返回 internal 错误，非 transient，冷却 600s）。
    let _ = c.compress(messages, 8_000, None).await.unwrap();

    // cooldown 期间应阻止压缩（即使远超阈值）。
    assert!(
        !c.should_compress(8_000).await,
        "Phase 3 失败后应进入冷却，阻止 further 压缩"
    );
    // 状态中 cooldown_until 已被设置。
    let s = c.state.read().await;
    assert!(s.summary_failure_cooldown_until.is_some());
}

#[tokio::test]
async fn cooldown_recovers_after_expiry() {
    let c = build_compressor(MockSummaryLlm::ok("{}"));
    // 手动设置 cooldown 到过去（已过期）。
    {
        let mut s = c.state.write().await;
        s.summary_failure_cooldown_until = Some(Instant::now() - Duration::from_secs(1));
    }
    // 已过期 → should_compress 应恢复基于阈值/anti-thrashing 的判断。
    assert!(
        c.should_compress(8_000).await,
        "cooldown 过期后应允许压缩"
    );
    assert!(
        !c.should_compress(4_000).await,
        "cooldown 过期后仍受阈值约束"
    );
}

#[tokio::test]
async fn cooldown_uses_short_duration_for_transient_errors() {
    // transient 错误（network_timeout）应设置短冷却（30s）。
    let c = build_compressor(MockSummaryLlm::err_transient());
    let messages: Vec<Message> = (0..30).map(|i| user_msg(&format!("m{i}"))).collect();
    let _ = c.compress(messages, 8_000, None).await.unwrap();

    // 验证冷却截止时间在 30s 后（而非 600s 后）。
    let s = c.state.read().await;
    let until = s.summary_failure_cooldown_until.expect("cooldown 应被设置");
    let now = Instant::now();
    // now < until <= now + 31s（容忍 1s 误差）。
    assert!(until > now, "cooldown 应在未来");
    assert!(
        until <= now + Duration::from_secs(31),
        "transient 错误冷却应为 30s（COOLDOWN_TRANSIENT_SECS）"
    );
}

// ---------- SubTask 11.4: tool pair integrity 集成 ----------

#[tokio::test]
async fn compress_invokes_sanitize_tool_pairs_for_orphan_pairs() {
    let summary_json = r#"{"active_task":"t","completed_actions":[],"key_decisions":[],"pending_questions":[],"relevant_context":""}"#;
    let c = build_compressor(MockSummaryLlm::ok(summary_json));

    // 构造：assistant 带 tool_call 在 head（前 3 条），但对应 tool 结果在 middle
    // （会被压缩到 summary 中），压缩后 assistant 的 tool_call 成为孤儿。
    let mut messages = vec![
        user_msg("start"),                          // 0 (head)
        assistant_with_tool_call("tc1", "search"),   // 1 (head) - tool_call 待修复
        user_msg("continue"),                        // 2 (head)
        tool_msg("tc1", "result"),                   // 3 (middle, 会被压缩掉)
    ];
    // 扩展到 30 条以触发 Phase 2/3 完整压缩流程。
    while messages.len() < 30 {
        messages.push(user_msg(&format!("extra{}", messages.len())));
    }

    let result = c.compress(messages, 8_000, None).await.unwrap();

    // 验证：压缩后没有孤儿 tool 消息（tool_msg 在 middle 被压缩，已不在 result）。
    let has_orphan_tool = result.iter().any(|m| m.role == "tool");
    assert!(
        !has_orphan_tool,
        "compress 末尾应调用 sanitize_tool_pairs 清除孤儿 tool 消息"
    );
    // 验证：head 中的 assistant 带 tool_call 但无对应 tool 结果 → tool_calls 应被清空。
    let orphan_assistant = result
        .iter()
        .find(|m| m.role == "assistant" && m.tool_calls.is_some());
    assert!(
        orphan_assistant.is_none(),
        "head 中孤儿 tool_calls 应被 sanitize_tool_pairs 移除"
    );
}

// ---------- SubTask 11.5: auto focus topic 集成 ----------

#[tokio::test]
async fn compress_auto_derives_focus_topic_when_none_provided() {
    let summary_json = r#"{"active_task":"t","completed_actions":[],"key_decisions":[],"pending_questions":[],"relevant_context":""}"#;
    let mock = CapturingSummaryLlm::ok(summary_json);
    let c = build_compressor(mock.clone());

    // 最近 3 条 user 消息中第一条（最早）= "m4"（索引 4，因为消息 0..29 中 user 在偶数索引？不，
    // 全部是 user 消息，所以最近 3 条是 m27/m28/m29，第一条 = m27）。
    // 实际：30 条全 user 消息，最近 3 条索引 27/28/29，"第一条"= 索引 27 = "m27"。
    let messages: Vec<Message> = (0..30).map(|i| user_msg(&format!("m{i}"))).collect();

    let _ = c.compress(messages, 8_000, None).await.unwrap();

    // 验证 LLM 收到的 user 参数包含 "Focus topic: m27"。
    let captured = mock.captured.lock().unwrap();
    assert!(
        captured
            .iter()
            .any(|(_, user)| user.contains("Focus topic: m27")),
        "未传 focus_topic 时应自动推断并注入 LLM"
    );
}

#[tokio::test]
async fn compress_uses_explicit_focus_topic_when_provided() {
    let summary_json = r#"{"active_task":"t","completed_actions":[],"key_decisions":[],"pending_questions":[],"relevant_context":""}"#;
    let mock = CapturingSummaryLlm::ok(summary_json);
    let c = build_compressor(mock.clone());

    let messages: Vec<Message> = (0..30).map(|i| user_msg(&format!("m{i}"))).collect();
    // 显式传入 focus_topic，应优先于自动推断。
    let _ = c
        .compress(messages, 8_000, Some("explicit topic"))
        .await
        .unwrap();

    let captured = mock.captured.lock().unwrap();
    assert!(
        captured
            .iter()
            .any(|(_, user)| user.contains("Focus topic: explicit topic")),
        "显式 focus_topic 应被注入 LLM"
    );
    // 同时不应出现自动推断的 "m27"。
    assert!(
        captured
            .iter()
            .all(|(_, user)| !user.contains("Focus topic: m27")),
        "显式 focus_topic 应优先于自动推断"
    );
}
