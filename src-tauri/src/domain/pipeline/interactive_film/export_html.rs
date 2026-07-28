//! ═══════════════════════════════════════════════════════════════════════════
//! Interactive Film Export HTML - HTML 导出
//! ═══════════════════════════════════════════════════════════════════════════
//!
//! build_playable_html：内嵌完整 JS player + CSS + GRAPH 数据。
//! Player 实现：变量初始化、condition 求值、effects 应用、可见选项过滤、
//! HUD 显示、ending 检测、重新开始。

use crate::shared::error::AppError;

use super::graph_schema::StoryGraph;

/// 构建 HTML 字符串中的 JS 安全转义（避免 `</script>` 注入）。
fn js_safe_json(json: &str) -> String {
    json.replace('<', "\\u003c")
}

pub fn build_playable_html(graph: &StoryGraph) -> Result<String, AppError> {
    let graph_json = serde_json::to_string(graph)?;
    let graph_js = js_safe_json(&graph_json);
    let title = html_escape(&graph.title);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="zh">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<style>
  :root {{
    --bg: #0f1115;
    --panel: #1a1d24;
    --text: #e8e8ea;
    --muted: #8a8f99;
    --accent: #6ea8fe;
    --danger: #ff6b6b;
    --good: #51cf66;
    --border: #2a2e38;
  }}
  * {{ box-sizing: border-box; }}
  body {{
    margin: 0;
    background: var(--bg);
    color: var(--text);
    font-family: -apple-system, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
    line-height: 1.7;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
  }}
  #hud {{
    background: var(--panel);
    border-bottom: 1px solid var(--border);
    padding: 8px 16px;
    font-size: 13px;
    color: var(--muted);
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }}
  #hud .var {{ background: var(--bg); padding: 2px 8px; border-radius: 4px; }}
  main {{
    max-width: 720px;
    margin: 0 auto;
    padding: 32px 20px 120px;
    flex: 1;
  }}
  #scene {{ white-space: pre-wrap; }}
  .dialogue {{ margin: 12px 0; padding: 8px 12px; border-left: 3px solid var(--accent); background: var(--panel); border-radius: 0 6px 6px 0; }}
  .dialogue .speaker {{ color: var(--accent); font-weight: 600; }}
  #choices {{ margin-top: 28px; display: flex; flex-direction: column; gap: 10px; }}
  .choice {{
    background: var(--panel);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 12px 16px;
    border-radius: 8px;
    cursor: pointer;
    text-align: left;
    font-size: 15px;
    transition: border-color .15s, background .15s;
  }}
  .choice:hover {{ border-color: var(--accent); background: #21252f; }}
  .choice .weight {{ font-size: 11px; color: var(--muted); margin-left: 8px; }}
  #ending {{
    margin-top: 32px;
    padding: 20px;
    border-radius: 10px;
    text-align: center;
  }}
  #ending.good {{ background: rgba(81,207,102,.12); border: 1px solid var(--good); }}
  #ending.bad {{ background: rgba(255,107,107,.12); border: 1px solid var(--danger); }}
  #ending.neutral {{ background: var(--panel); border: 1px solid var(--border); }}
  #ending.secret {{ background: rgba(110,168,254,.12); border: 1px solid var(--accent); }}
  #ending h2 {{ margin: 0 0 8px; }}
  #restart {{
    margin-top: 24px;
    background: transparent;
    border: 1px solid var(--accent);
    color: var(--accent);
    padding: 8px 16px;
    border-radius: 8px;
    cursor: pointer;
  }}
  #restart:hover {{ background: rgba(110,168,254,.1); }}
  .hidden {{ display: none !important; }}
</style>
</head>
<body>
<div id="hud"></div>
<main>
  <div id="scene"></div>
  <div id="dialogues"></div>
  <div id="choices"></div>
  <div id="ending" class="hidden"></div>
  <button id="restart" class="hidden">重新开始</button>
</main>
<script>
const GRAPH = {graph_js};

const TypeStart = "start";
const TypeEnding = "ending";

let state = {{}};
let currentNode = null;

function initState() {{
  state = {{}};
  for (const v of (GRAPH.variables || [])) {{
    state[v.name] = v.default;
  }}
}}

function findStart() {{
  return GRAPH.nodes.find(n => n.type === TypeStart) || GRAPH.nodes[0];
}}

function nodeById(id) {{
  return GRAPH.nodes.find(n => n.id === id);
}}

function endingByNodeId(id) {{
  return (GRAPH.endings || []).find(e => e.nodeId === id);
}}

function evaluateCondition(cond) {{
  if (!cond) return true;
  const lhs = state[cond.var];
  const rhs = cond.value;
  switch (cond.op) {{
    case "==": return lhs === rhs;
    case "!=": return lhs !== rhs;
    case ">=": return Number(lhs) >= Number(rhs);
    case "<=": return Number(lhs) <= Number(rhs);
    case ">":  return Number(lhs) > Number(rhs);
    case "<":  return Number(lhs) < Number(rhs);
    default:   return false;
  }}
}}

function applyEffects(effects) {{
  for (const e of (effects || [])) {{
    const cur = state[e.var];
    if (e.op === "set") {{
      state[e.var] = e.value;
    }} else {{
      const base = Number(cur); const d = Number(e.value);
      const next = (isNaN(base) ? 0 : base) + (isNaN(d) ? 0 : d) * (e.op === "sub" ? -1 : 1);
      state[e.var] = Number.isInteger(next) ? next : next;
    }}
  }}
}}

function visibleChoices(node) {{
  return (node.choices || []).filter(c => evaluateCondition(c.condition));
}}

function renderHUD() {{
  const hud = document.getElementById('hud');
  hud.innerHTML = '';
  for (const v of (GRAPH.variables || [])) {{
    const span = document.createElement('span');
    span.className = 'var';
    span.textContent = v.name + ': ' + state[v.name];
    hud.appendChild(span);
  }}
}}

function renderNode(node) {{
  currentNode = node;
  const sceneEl = document.getElementById('scene');
  sceneEl.textContent = node.sceneDesc || node.title || '';
  const dialEl = document.getElementById('dialogues');
  dialEl.innerHTML = '';
  for (const line of (node.dialogue || [])) {{
    const div = document.createElement('div');
    div.className = 'dialogue';
    const sp = document.createElement('div');
    sp.className = 'speaker';
    sp.textContent = line.speaker + (line.emotion ? '（' + line.emotion + '）' : '');
    const tx = document.createElement('div');
    tx.textContent = line.text;
    div.appendChild(sp); div.appendChild(tx);
    dialEl.appendChild(div);
  }}
  const choicesEl = document.getElementById('choices');
  choicesEl.innerHTML = '';
  const endingEl = document.getElementById('ending');
  endingEl.className = 'hidden';
  endingEl.innerHTML = '';
  document.getElementById('restart').classList.add('hidden');

  if (node.type === TypeEnding) {{
    const ending = endingByNodeId(node.id);
    if (ending) {{
      endingEl.className = ending.type || 'neutral';
      endingEl.innerHTML = '<h2>' + esc(ending.title) + '</h2><p>' + esc(ending.description || '') + '</p>';
    }} else {{
      endingEl.className = 'neutral';
      endingEl.innerHTML = '<h2>结局</h2>';
    }}
    document.getElementById('restart').classList.remove('hidden');
    renderHUD();
    return;
  }}

  const choices = visibleChoices(node);
  if (choices.length === 0) {{
    const btn = document.createElement('div');
    btn.textContent = '（没有可选的选项 —— 死路）';
    btn.style.color = 'var(--muted)';
    choicesEl.appendChild(btn);
    document.getElementById('restart').classList.remove('hidden');
    renderHUD();
    return;
  }}
  for (const c of choices) {{
    const btn = document.createElement('button');
    btn.className = 'choice';
    btn.innerHTML = esc(c.text) + (c.weight ? '<span class="weight">' + c.weight + '</span>' : '');
    btn.addEventListener('click', () => onChoice(c));
    choicesEl.appendChild(btn);
  }}
  renderHUD();
}}

function onChoice(choice) {{
  applyEffects(choice.effects);
  const next = nodeById(choice.targetNodeId);
  if (!next) {{
    console.error('missing target node:', choice.targetNodeId);
    return;
  }}
  renderNode(next);
}}

function restart() {{
  initState();
  renderNode(findStart());
}}

function esc(s) {{
  const d = document.createElement('div');
  d.textContent = s == null ? '' : String(s);
  return d.innerHTML;
}}

initState();
renderNode(findStart());
document.getElementById('restart').addEventListener('click', restart);
</script>
</body>
</html>"#
    );
    Ok(html)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::pipeline::interactive_film::graph_schema::*;

    fn sample_graph() -> StoryGraph {
        StoryGraph {
            schema_version: 1,
            project_id: "p".into(),
            title: "测试</script><script>alert(1)</script>".into(),
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
                            condition: None,
                            effects: vec![],
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
            endings: vec![Ending {
                id: "e1".into(),
                node_id: "end".into(),
                title: "完结".into(),
                ending_type: EndingType::Good,
                description: "好的结局".into(),
            }],
            world_anchor: None,
            characters: vec![],
            variables: vec![],
        }
    }

    #[test]
    fn builds_html_and_escapes_script_injection() {
        let g = sample_graph();
        let html = build_playable_html(&g).unwrap();
        // 标题中的 </script> 应被转义为 \u003c/script>（GRAPH 数据内）或 &lt;/script&gt;（title 元素）
        assert!(html.contains("测试"));
        assert!(!html.contains("</script><script>alert(1)</script>"));
        assert!(html.contains("<script>"));
    }
}
