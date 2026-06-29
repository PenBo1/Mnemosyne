//! Writing methodology utilities.

/// Build a writing methodology section for the writer prompt
pub fn build_writing_methodology_section(language: &str) -> String {
    match language {
        "en" => {
            r#"## Writing Methodology

### Scene Construction
- Every scene must have: a character with a goal, an obstacle, and a concrete outcome
- Open with action or dialogue, not exposition
- End scenes with a shift in information, pressure, or relationship

### Dialogue
- Each character must have a distinct voice
- Dialogue must advance plot or reveal character
- Avoid on-the-nose dialogue (characters stating their feelings directly)

### Pacing
- Alternate between high-tension and low-tension scenes
- Use paragraph length to control rhythm: short for tension, long for description
- Every 3-5 chapters should have a mini-goal achievement or escalation

### Hook Management
- Plant hooks early, advance them regularly
- Never open more hooks than you close
- Each hook must have a clear payoff timeline"#
        }
        _ => {
            r#"## 写作方法论

### 场景构建
- 每个场景必须有：有目标的角色、障碍、具体结果
- 以动作或对话开头，不要以说明开头
- 场景结尾要有信息、压力或关系的变化

### 对话
- 每个角色必须有独特的说话风格
- 对话必须推进剧情或揭示角色
- 避免直白对话（角色直接说出自己的感受）

### 节奏
- 高紧张和低紧张场景交替使用
- 用段落长度控制节奏：短段落制造紧张，长段落用于描写
- 每 3-5 章应有一个小目标达成或悬念升级

### 伏笔管理
- 早埋伏笔，定期推进
- 不要开比收更多的伏笔
- 每个伏笔必须有明确的兑现时间线"#
        }
    }.to_string()
}
