use crate::infra::llm::StreamEvent;

pub struct StreamState {
    pub assistant_content: String,
    pub tool_calls: Vec<(String, String, String)>,
    pub current_tool_id: Option<String>,
    pub current_tool_name: String,
    pub current_tool_args: String,
    pub total_input: u32,
    pub total_output: u32,
}

pub enum StreamAction {
    Delta(String),
    ToolCallBegin {
        tool_call_id: String,
        tool: String,
        args: String,
    },
    Finish { tool_calls_empty: bool },
    Error(String),
}

impl StreamState {
    pub fn new() -> Self {
        Self {
            assistant_content: String::new(),
            tool_calls: Vec::new(),
            current_tool_id: None,
            current_tool_name: String::new(),
            current_tool_args: String::new(),
            total_input: 0,
            total_output: 0,
        }
    }

    pub fn process_event(&mut self, event: StreamEvent) -> StreamAction {
        match event {
            StreamEvent::TextDelta { content } => {
                tracing::debug!(content_len = content.len(), "StreamEvent::TextDelta");
                self.assistant_content.push_str(&content);
                StreamAction::Delta(content)
            }
            StreamEvent::ToolCallStart { id, name } => {
                tracing::info!(tool_id = %id, tool_name = %name, "StreamEvent::ToolCallStart");
                self.current_tool_id = Some(id.clone());
                self.current_tool_name = name;
                self.current_tool_args.clear();
                StreamAction::Delta(String::new())
            }
            StreamEvent::ToolCallDelta {
                id: _,
                args_delta,
            } => {
                self.current_tool_args.push_str(&args_delta);
                StreamAction::Delta(String::new())
            }
            StreamEvent::ToolCallEnd { id: _ } => {
                if let Some(tool_id) = self.current_tool_id.take() {
                    let tool = self.current_tool_name.clone();
                    let args = self.current_tool_args.clone();
                    self.tool_calls.push((tool_id.clone(), tool.clone(), args.clone()));
                    StreamAction::ToolCallBegin {
                        tool_call_id: tool_id,
                        tool,
                        args,
                    }
                } else {
                    StreamAction::Delta(String::new())
                }
            }
            StreamEvent::Finish { reason: _, usage } => {
                tracing::info!(input = usage.input_tokens, output = usage.output_tokens, "StreamEvent::Finish");
                self.total_input += usage.input_tokens;
                self.total_output += usage.output_tokens;
                StreamAction::Finish {
                    tool_calls_empty: self.tool_calls.is_empty(),
                }
            }
            StreamEvent::Error(e) => {
                tracing::error!(error = %e, "StreamEvent::Error");
                StreamAction::Error(format!("Stream error: {}", e))
            }
        }
    }

    pub fn build_tool_call_json(&self) -> String {
        serde_json::to_string(
            &self.tool_calls
                .iter()
                .map(|(id, name, args)| {
                    serde_json::json!({
                        "id": id,
                        "name": name,
                        "arguments": args
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default()
    }
}
