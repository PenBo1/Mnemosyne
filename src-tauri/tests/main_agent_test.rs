use mnemosyne_lib::domain::agents::main_agent::{types::*, safety_gate::SafetyGate};
use serde_json::json;

#[test]
fn test_risk_levels() {
    // Safe tools
    assert_eq!(SafetyGate::evaluate_risk("read_file", &json!({"path": "test.txt"})), RiskLevel::Safe);
    assert_eq!(SafetyGate::evaluate_risk("list_files", &json!({"path": "."})), RiskLevel::Safe);
    assert_eq!(SafetyGate::evaluate_risk("search_memory", &json!({"query": "test"})), RiskLevel::Safe);

    // Moderate tools
    assert_eq!(SafetyGate::evaluate_risk("archive_memory", &json!({"content": "test"})), RiskLevel::Moderate);
    assert_eq!(SafetyGate::evaluate_risk("write_file", &json!({"path": "src/main.rs", "content": "test"})), RiskLevel::Moderate);

    // High risk bash
    assert_eq!(SafetyGate::evaluate_risk("bash", &json!({"command": "rm -rf /"})), RiskLevel::High);
    assert_eq!(SafetyGate::evaluate_risk("bash", &json!({"command": "sudo apt install"})), RiskLevel::High);
    assert_eq!(SafetyGate::evaluate_risk("bash", &json!({"command": "git push --force"})), RiskLevel::High);

    // Moderate risk bash
    assert_eq!(SafetyGate::evaluate_risk("bash", &json!({"command": "git commit -m test"})), RiskLevel::Moderate);

    // High risk write paths
    assert_eq!(SafetyGate::evaluate_risk("write_file", &json!({"path": "/etc/passwd", "content": "x"})), RiskLevel::High);
    assert_eq!(SafetyGate::evaluate_risk("write_file", &json!({"path": ".env", "content": "x"})), RiskLevel::High);
}

#[test]
fn test_confirmation_request_generation() {
    let req = SafetyGate::create_confirmation_request(
        1,
        "bash",
        &json!({"command": "rm -rf /tmp/test"}),
    );
    assert_eq!(req.step_id, 1);
    assert_eq!(req.risk_level, RiskLevel::High);
    assert!(req.description.contains("bash"));
    assert!(req.details.contains("rm -rf /tmp/test"));
}

#[test]
fn test_plan_step_serialization() {
    let step = PlanStep {
        id: 1,
        description: "Read config".to_string(),
        tool_name: Some("read_file".to_string()),
        tool_args: Some(json!({"path": "config.json"})),
        risk_level: RiskLevel::Safe,
        status: StepStatus::Pending,
        result: None,
    };

    let json = serde_json::to_string(&step).unwrap();
    let deserialized: PlanStep = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, 1);
    assert_eq!(deserialized.tool_name, Some("read_file".to_string()));
    assert_eq!(deserialized.status, StepStatus::Pending);
}

#[test]
fn test_agent_status_serialization() {
    let status = AgentStatus::Executing;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"Executing\"");
}

#[test]
fn test_progress_update_serialization() {
    let update = ProgressUpdate {
        status: AgentStatus::Planning,
        current_step: Some(1),
        total_steps: Some(5),
        message: "Creating plan...".to_string(),
    };

    let json = serde_json::to_string(&update).unwrap();
    assert!(json.contains("Planning"));
    assert!(json.contains("Creating plan"));
}

#[test]
fn test_conversation_message() {
    let msg = ConversationMessage {
        role: MessageRole::User,
        content: "Write me a story".to_string(),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: ConversationMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.role, MessageRole::User);
    assert_eq!(deserialized.content, "Write me a story");
}
