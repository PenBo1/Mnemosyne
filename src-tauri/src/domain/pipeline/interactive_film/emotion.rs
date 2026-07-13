// 情感分析。
//
// 任务规格的情感分析设计：
// - analyze_text_emotion：在文本中搜索情感词，否定词守卫翻转极性，累加得分
// - analyze_emotional_arc：沿路径收集 scene_desc + dialogue text，逐节点计算情感分数

use super::graph_schema::StoryGraph;
use super::paths::RuntimePath;

/// 情感词典条目
struct EmotionWord {
    word: &'static str,
    valence: f64, // [-1, 1]
}

/// 中文情感词典（20 词）
const EMOTION_LEXICON: &[EmotionWord] = &[
    EmotionWord { word: "开心", valence: 0.8 },
    EmotionWord { word: "快乐", valence: 0.8 },
    EmotionWord { word: "高兴", valence: 0.7 },
    EmotionWord { word: "兴奋", valence: 0.8 },
    EmotionWord { word: "满足", valence: 0.6 },
    EmotionWord { word: "喜欢", valence: 0.7 },
    EmotionWord { word: "爱", valence: 0.9 },
    EmotionWord { word: "温暖", valence: 0.6 },
    EmotionWord { word: "希望", valence: 0.6 },
    EmotionWord { word: "安心", valence: 0.5 },
    EmotionWord { word: "伤心", valence: -0.7 },
    EmotionWord { word: "难过", valence: -0.7 },
    EmotionWord { word: "悲伤", valence: -0.8 },
    EmotionWord { word: "痛苦", valence: -0.9 },
    EmotionWord { word: "愤怒", valence: -0.8 },
    EmotionWord { word: "害怕", valence: -0.7 },
    EmotionWord { word: "恐惧", valence: -0.8 },
    EmotionWord { word: "绝望", valence: -0.9 },
    EmotionWord { word: "孤独", valence: -0.6 },
    EmotionWord { word: "失望", valence: -0.6 },
];

/// 否定词守卫：词前一个字符若为这些则翻转极性
const NEGATION_PREFIXES: &[char] = &['不', '没', '无', '别', '未'];

/// 判断一个字符是否为否定词。
fn is_negation(ch: char) -> bool {
    NEGATION_PREFIXES.contains(&ch)
}

/// 计算文本情感分数。
///
/// 遍历情感词典，在 text 中搜索每个词；
/// 检查词前一个字符是否是否定词，若是则翻转极性（valence *= -1）；
/// 累加所有匹配词的 valence，返回总分。
pub fn analyze_text_emotion(text: &str) -> f64 {
    let chars: Vec<char> = text.chars().collect();
    let mut total: f64 = 0.0;
    for entry in EMOTION_LEXICON {
        let word_chars: Vec<char> = entry.word.chars().collect();
        let word_len = word_chars.len();
        if word_len == 0 || word_len > chars.len() {
            continue;
        }
        // 在 chars 中搜索所有出现位置
        let mut i: usize = 0;
        while i + word_len <= chars.len() {
            let matches = chars[i..i + word_len] == word_chars[..];
            if matches {
                // 检查前一个字符是否为否定词
                let negated = i > 0 && is_negation(chars[i - 1]);
                if negated {
                    total -= entry.valence;
                } else {
                    total += entry.valence;
                }
            }
            i += 1;
        }
    }
    total
}

/// 情感弧线点
#[derive(Debug, Clone)]
pub struct EmotionArcPoint {
    pub node_id: String,
    pub score: f64,
}

/// 分析路径的情感弧线。
///
/// 遍历 path.node_ids，对每个节点收集 scene_desc + dialogue 的所有 text，
/// 调用 analyze_text_emotion 计算分数。
pub fn analyze_emotional_arc(path: &RuntimePath, graph: &StoryGraph) -> Vec<EmotionArcPoint> {
    let node_by_id: std::collections::HashMap<&str, &super::graph_schema::StoryNode> = graph
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n))
        .collect();

    path.node_ids
        .iter()
        .map(|node_id| {
            let score = node_by_id
                .get(node_id.as_str())
                .map(|node| {
                    let mut buf = String::new();
                    if !node.scene_desc.is_empty() {
                        buf.push_str(&node.scene_desc);
                    }
                    for line in &node.dialogue {
                        if !buf.is_empty() {
                            buf.push(' ');
                        }
                        buf.push_str(&line.text);
                    }
                    analyze_text_emotion(&buf)
                })
                .unwrap_or(0.0);
            EmotionArcPoint {
                node_id: node_id.clone(),
                score,
            }
        })
        .collect()
}
