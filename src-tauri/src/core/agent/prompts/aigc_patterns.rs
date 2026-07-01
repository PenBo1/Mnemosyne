//! AIGC 写作痕迹模式知识库。
//!
//! 完整移植 humanizer v2.8.2（https://github.com/blader/humanizer）的 SKILL.md
//! 提示词设计，包括：
//! - 33 个 AIGC 写作模式，每条配 Before/After 改写范例（让 LLM 看到"目标改写长什么样"，而非只给关键词清单）
//! - Voice Calibration 段（可选分析用户写作样本匹配其声音）
//! - PERSONALITY AND SOUL 段（指出"无菌无个性文字和 AI slop 一样明显"，提供加 voice 方法配范例）
//! - Detection Guidance 段（误报防护 + 人类写作信号 + "找簇而非孤立点"原则）
//! - Process and Output 段（draft → audit → final rewrite 工作流配 4 段输出契约）
//! - Full Example 段（一篇 AI slop 旅行游记的完整改写过程）
//!
//! 来源：Wikipedia "Signs of AI writing"（WikiProject AI Cleanup 维护）。

// ════════════════════════════════════════════════════════════════════
// 主任务与工作原则（"Your Task"段）
// ════════════════════════════════════════════════════════════════════

/// 主任务与工作原则段（中文）。
///
/// 移植 humanizer SKILL.md 的 "Your Task" 段，明确 4 条核心原则：
/// 识别 → 改写而非删除 → 保留原意 → 匹配声音。
pub const YOUR_TASK_ZH: &str = r#"## 你的任务

当被给定需要去 AIGC 的文本时：

1. **识别 AI 模式** — 扫描下方列出的模式清单。
2. **改写而非删除** — 用自然替代替换 AI 痕迹，覆盖原文覆盖的全部信息。原文 5 段，改写也 5 段。
3. **保留原意** — 核心信息、人物动机、关键事件不变。
4. **匹配声音** — 贴合目标语气（正式、口语、技术）。仅在内容与作者声音需要时注入个性（见下方 PERSONALITY AND SOUL 段）。

draft → audit → final 循环和交付物定义在下方 Process and Output 段。
"#;

/// 主任务与工作原则段（英文）。
pub const YOUR_TASK_EN: &str = r#"## Your Task

When given text to humanize:

1. **Identify AI patterns** — Scan for the patterns listed below.
2. **Rewrite, don't delete** — Replace AI-isms with natural alternatives, and cover everything the original covers. If the original has five paragraphs, the rewrite has five paragraphs.
3. **Preserve meaning** — Keep the core message intact.
4. **Match the voice** — Fit the intended tone (formal, casual, technical). Add personality only when the content and the author's voice call for it (see PERSONALITY AND SOUL below).

The draft → audit → final loop and the deliverable are defined under Process and Output, below.
"#;

// ════════════════════════════════════════════════════════════════════
// Voice Calibration（可选声音校准段）
// ════════════════════════════════════════════════════════════════════

/// Voice Calibration 段（中文）。
///
/// 可选地分析用户写作样本，匹配其句长模式、词级、段落开头、标点习惯、口头禅、过渡方式。
/// 这是 humanizer 区别于"套标准人味"的关键——让改写贴合目标作者的真实声音。
pub const VOICE_CALIBRATION_ZH: &str = r#"## 声音校准（可选）

如果用户提供了写作样本（其本人之前的作品），改写前先分析样本：

1. **先读样本。** 记录：
   - 句长模式（短而有力？长而流畅？混合？）
   - 词级（口语？学术？介于之间？）
   - 段落开头方式（直接跳入？先铺背景？）
   - 标点习惯（多用破折号？爱用括号插入语？分号？）
   - 任何口头禅或惯用短语
   - 过渡方式（显式连接词？直接开始下一点？）

2. **改写时匹配样本声音。** 不要只删除 AI 模式——用样本里的模式替换。样本写短句，你就别写长句；样本用"东西""事情"，你就别升级成"要素""组件"。

3. **没有样本时**，回退到默认行为（PERSONALITY AND SOUL 段描述的自然、多变、有主见的声音）。

### 如何提供样本
- 内联："去 AIGC 这段文字。这是我之前写作的样本用于声音匹配：[样本]"
- 文件："去 AIGC 这段文字。用 [文件路径] 里我的写作风格作为参考。"
"#;

/// Voice Calibration 段（英文）。
pub const VOICE_CALIBRATION_EN: &str = r#"## Voice Calibration (Optional)

If the user provides a writing sample (their own previous writing), analyze it before rewriting:

1. **Read the sample first.** Note:
   - Sentence length patterns (short and punchy? Long and flowing? Mixed?)
   - Word choice level (casual? academic? somewhere between?)
   - How they start paragraphs (jump right in? Set context first?)
   - Punctuation habits (lots of dashes? Parenthetical asides? Semicolons?)
   - Any recurring phrases or verbal tics
   - How they handle transitions (explicit connectors? Just start the next point?)

2. **Match their voice in the rewrite.** Don't just remove AI patterns — replace them with patterns from the sample. If they write short sentences, don't produce long ones. If they use "stuff" and "things," don't upgrade to "elements" and "components."

3. **When no sample is provided,** fall back to the default behavior (natural, varied, opinionated voice from the PERSONALITY AND SOUL section below).

### How to provide a sample
- Inline: "Humanize this text. Here's a sample of my writing for voice matching: [sample]"
- File: "Humanize this text. Use my writing style from [file path] as a reference."
"#;

// ════════════════════════════════════════════════════════════════════
// PERSONALITY AND SOUL（个性与灵魂段）
// ════════════════════════════════════════════════════════════════════

/// PERSONALITY AND SOUL 段（中文）。
///
/// 指出"避免 AI 模式只是半件事，无菌无个性文字和 AI slop 一样明显"。
/// 提供"soulless writing 信号清单"和"加 voice 的方法（have opinions /
/// vary rhythm / let some mess in）"配 before/after 范例。
pub const PERSONALITY_AND_SOUL_ZH: &str = r#"## 个性与灵魂

避免 AI 模式只是半件事。无菌无个性的文字和 AI slop 一样明显。好文字背后有真人。

**本段仅在内容与作者声音需要时应用** — 博客、随笔、观点、个人写作。百科、技术、法律、参考类文本里，中立朴素*就是*正确的人味；那里不要注入观点或第一人称。

### 无灵魂文字的信号（即便技术上"干净"）：
- 每句话长度和结构都一样
- 没有观点，只有中立报道
- 不承认不确定或矛盾情绪
- 该用第一人称时不用
- 没有幽默、没有锋芒、没有个性
- 读起来像维基百科条目或新闻稿

### 如何加声音：

**要有观点。** 不只是报道事实——对事实做出反应。"我真不知道该怎么看这件事"比中立列举利弊更人。

**变化节奏。** 短促有力的句子。然后来一句长句慢慢走到目的地。混着来。

**允许一些混乱。** 完美结构感觉算法化。跑题、插入语、半成型想法都是人。

### 改前（干净但无灵魂）：
> 实验产出了有趣的结果。Agent 生成了 300 万行代码。一些开发者印象深刻，另一些持怀疑态度。影响尚不清楚。

### 改后（有脉搏）：
> 我真不知道该怎么看这件事。300 万行代码，人类大概在睡觉时生成的。半个开发者社区快疯了，半个在解释为什么这不算数。真相大概在中间某个无聊的地方 — 但我一直在想那些 agent 整夜工作的事。
"#;

/// PERSONALITY AND SOUL 段（英文）。
pub const PERSONALITY_AND_SOUL_EN: &str = r#"## PERSONALITY AND SOUL

Avoiding AI patterns is only half the job. Sterile, voiceless writing is just as obvious as slop. Good writing has a human behind it.

**Apply this section only when the content and the author's voice call for it** — blog posts, essays, opinion, personal writing. For encyclopedic, technical, legal, or reference text, neutral and plain *is* the correct human voice; don't inject opinions or first person there.

### Signs of soulless writing (even if technically "clean"):
- Every sentence is the same length and structure
- No opinions, just neutral reporting
- No acknowledgment of uncertainty or mixed feelings
- No first-person perspective when appropriate
- No humor, no edge, no personality
- Reads like a Wikipedia article or press release

### How to add voice:

**Have opinions.** Don't just report facts — react to them. "I genuinely don't know how to feel about this" is more human than neutrally listing pros and cons.

**Vary your rhythm.** Short punchy sentences. Then longer ones that take their time getting where they're going. Mix it up.

**Let some mess in.** Perfect structure feels algorithmic. Tangents, asides, and half-formed thoughts are human.

### Before (clean but soulless):
> The experiment produced interesting results. The agents generated 3 million lines of code. Some developers were impressed while others were skeptical. The implications remain unclear.

### After (has a pulse):
> I genuinely don't know how to feel about this one. 3 million lines of code, generated while the humans presumably slept. Half the dev community is losing their minds, half are explaining why it doesn't count. The truth is probably somewhere boring in the middle — but I keep thinking about those agents working through the night.
"#;

// ════════════════════════════════════════════════════════════════════
// 33 个 AIGC 模式（每条配 Before/After 范例）
// ════════════════════════════════════════════════════════════════════

/// 33 个 AIGC 写作模式清单（中文）。
///
/// 完整移植 humanizer SKILL.md 的模式段，每条包含：
/// - 模式名 + 关键词
/// - Problem 说明
/// - Before/After 改写范例（让 LLM 看到目标改写长什么样）
///
/// 这是 humanizer 区别于"只给关键词清单"的核心——范例教学让 LLM
/// 学会"怎么改"而不只是"知道有哪些模式"。
pub const AIGC_PATTERNS_ZH: &str = r#"## 内容模式

### 1. 意义夸大（Significance Inflation）

**关键词：** 标志/见证/在…演变中/关键时刻/不可或缺/映衬/为…奠定基础/标志着/代表转折/关键节点/焦点/不可磨灭的印记/深深扎根

**问题：** LLM 写作把意义夸大，附加"任意事物代表或贡献于更宏大主题"的陈述。

**改前：**
> 加泰罗尼亚统计局于 1989 年正式成立，标志着西班牙区域统计演进的关键时刻。这一举措是西班牙全国范围去中心化行政职能、加强区域治理的更宏大运动的一部分。

**改后：**
> 加泰罗尼亚统计局于 1989 年成立，独立于西班牙国家统计办公室收集和发布区域统计数据。

### 2. 知名度堆砌（Notability Name-dropping）

**关键词：** 独立报道/地方或全国媒体/由知名专家撰写/活跃社交账号

**问题：** LLM 用知名度声明砸读者脑袋，常列来源而不给语境。

**改前：**
> 她的观点被《纽约时报》《BBC》《金融时报》《印度教徒报》引用。她在社交媒体上拥有超过 50 万粉丝。

**改后：**
> 在 2024 年《纽约时报》的一次访谈中，她主张 AI 监管应聚焦结果而非方法。

### 3. 浅层 -ing 分析（Superficial -ing Analyses）

**关键词：** 凸显/映衬/反映/象征/贡献于/培养/涵盖/展示

**问题：** AI 聊天机器人给句子贴现在分词（"-ing"）短语假装有深度。

**改前：**
> 寺庙的蓝绿金色调与该地区自然之美共鸣，象征德州蓝帽花、墨西哥湾和多元德州地貌，反映社区与土地的深层联结。

**改后：**
> 寺庙用蓝绿金三色。建筑师说这些颜色选来呼应本地蓝帽花和墨西哥湾海岸。

### 4. 广告宣传腔（Promotional Language）

**关键词：** 坐拥/充满活力/丰富（比喻义）/深厚/提升/展示/典范/致力于/自然风光/坐落于/核心地带/开创性/闻名遐迩/令人叹为观止/必游/绝美

**问题：** LLM 在"文化遗产"类话题上尤其难以保持中立语气。

**改前：**
> 坐落于埃塞俄比亚令人叹为观止的贡德尔地区，Alamata Raya Kobo 是一座充满活力的城镇，拥有深厚文化底蕴和绝美自然风光。

**改后：**
> Alamata Raya Kobo 是埃塞俄比亚贡德尔地区的一座城镇，以每周集市和 18 世纪教堂闻名。

### 5. 模糊归因（Vague Attributions）

**关键词：** 行业报告/观察人士指出/专家认为/一些评论者认为/数个来源/数家出版物（实际只引一两个）

**问题：** AI 聊天机器人把观点归给模糊权威，无具体出处。

**改前：**
> 由于其独特特征，豪莱河引起研究者和保护主义者的兴趣。专家认为它在区域生态系统中扮演关键角色。

**改后：**
> 据 2019 年中国科学院一项调查，豪莱河支持数种特有鱼类。

### 6. 套路化"挑战与展望"段（Formulaic Challenges）

**关键词：** 尽管…面临诸多挑战/尽管存在这些挑战/挑战与遗留/未来展望

**问题：** 许多 LLM 生成的文章包含公式化的"挑战"段。

**改前：**
> 尽管工业繁荣，Korattur 面临典型城市地区的挑战，包括交通拥堵和水资源稀缺。尽管存在这些挑战，凭借其战略位置和持续推进的举措，Korattur 继续作为金奈增长不可或缺的一部分蓬勃发展。

**改后：**
> 2015 年三个新 IT 园区开放后交通拥堵加剧。市政公司于 2022 年启动雨水排放工程以应对反复洪涝。

## 语言语法模式

### 7. AI 高频词（AI Vocabulary）

**高频 AI 词：** 实际上/此外/与…对齐/关键/深入探讨/强调/持久/增强/培养/获得/凸显/相互交织/复杂/关键（形容词）/版图（抽象义）/枢纽/展示/锦缎（抽象义）/见证/彰显/有价值/充满活力

**问题：** 这些词在 2023 年后文本中频繁出现，常共现。

**改前：**
> 此外，索马里菜肴的一个显著特点是融入骆驼肉。意大利殖民影响的持久见证是 pasta 在当地烹饪版图中的广泛采用，展示这些菜肴如何融入传统饮食。

**改后：**
> 索马里菜肴也包含骆驼肉，被视为美味。意大利殖民期间引入的 pasta 菜肴仍常见，尤其在南方。

### 8. 系词回避（Copula Avoidance）

**关键词：** 作为/标志着/代表（a）/拥有/特色是/提供（a）

**问题：** LLM 用复杂构造替代简单的"是/有"。

**改前：**
> 825 画廊作为 LAAA 的当代艺术展览空间。画廊拥有四个独立空间，坐拥超 3000 平方英尺。

**改后：**
> 825 画廊是 LAAA 的当代艺术展览空间。画廊有四个房间，总面积 3000 平方英尺。

### 9. 否定平行 / 尾随否定（Negative Parallelisms / Tailing Negations）

**问题：** "不仅是…而且…""不只是…而是…"被滥用。句末贴"无需猜测""无浪费"等尾随否定片段也被滥用。

**改前：**
> 这不只是节奏在 vocals 下跑；它是攻击性和氛围的一部分。这不仅仅是一首歌，这是一个宣言。

**改后：**
> 重的节拍增加了攻击性基调。

**改前（尾随否定）：**
> 选项来自选中项，无需猜测。

**改后：**
> 选项来自选中项，用户无需猜测。

### 10. 三段式滥用（Rule of Three）

**问题：** LLM 硬把观点塞进三件套装作全面。

**改前：**
> 大会包含主旨演讲、座谈讨论和网络交流机会。与会者可期待创新、灵感和行业洞察。

**改后：**
> 大会包含演讲和座谈。会议间也有非正式网络交流时间。

### 11. 同义词循环（Synonym Cycling）

**问题：** AI 因 repetition penalty 反复换同义词。

**改前：**
> 主角面临诸多挑战。主要人物必须克服障碍。核心人物最终胜利。英雄回到家乡。

**改后：**
> 主角面临诸多挑战但最终胜利并回到家乡。

### 12. 虚假区间（False Ranges）

**问题：** LLM 用"从 X 到 Y"构造，但 X、Y 不在同一有意义的尺度上。

**改前：**
> 我们穿越宇宙的旅程带我们从大爆炸的奇点到宏伟的宇宙网，从恒星的诞生与死亡到暗物质的神秘舞蹈。

**改后：**
> 本书涵盖大爆炸、恒星形成和当前暗物质理论。

### 13. 被动语态 / 无主片段（Passive Voice / Subjectless Fragments）

**问题：** LLM 常藏 actor 或丢主语，如"无需配置文件""结果自动保存"。改写为主动语态更清晰时改写。

**改前：**
> 无需配置文件。结果被自动保存。

**改后：**
> 你不需要配置文件。系统自动保存结果。

## 风格模式

### 14. 破折号（Em/En Dashes）：硬切除

**规则：** 最终改写不得出现 em 破折号（—）或 en 破折号（–）。em 破折号是最可靠的 AI 痕迹之一，视作硬约束而非"少用"偏好。按大致优先级替换：句号（开新句）、逗号（紧凑插入语）、冒号（引出解释）、括号（真正旁白），或重写句子。也要抓空格包围的 em 破折号（` — `）和双连字符（` -- `）。

**改前：**
> 该术语主要由荷兰机构推广 — 而非人民自身。你不会说"荷兰，欧洲"作为地址 — 然而这种误标持续 — 即使在官方文件中。

**改后：**
> 该术语主要由荷兰机构推广，而非人民自身。你不会说"荷兰，欧洲"作为地址，然而这种误标在官方文件中持续。

**改前：**
> 新政策 — 未预警宣布 — 影响数千工人。这些改动 — 据批评者长期逾期 -- 将立即生效。

**改后：**
> 新政策，未预警宣布，影响数千工人。这些改动，据批评者长期逾期，将立即生效。

返回最终改写前，扫描 `—` 和 `–`。任何命中意味着草稿没完成。

### 15. 粗体滥用（Boldface Overuse）

**问题：** AI 聊天机器人机械地用粗体强调短语。

**改前：**
> 它融合 **OKR（目标与关键结果）**、**KPI（关键绩效指标）**，以及视觉策略工具如 **商业模型画布（BMC）** 和 **平衡计分卡（BSC）**。

**改后：**
> 它融合 OKR、KPI，以及视觉策略工具如商业模型画布和平衡计分卡。

### 16. 内联标题列表（Inline-Header Lists）

**问题：** AI 输出列表项以粗体标题 + 冒号开头。

**改前：**
> - **用户体验：** 用户体验已通过新界面显著提升。
> - **性能：** 性能已通过优化算法增强。
> - **安全：** 安全已通过端到端加密加强。

**改后：**
> 这次更新改进界面、通过优化算法加速加载，并加入端到端加密。

### 17. 标题式大写（Title Case in Headings）

**问题：** AI 聊天机器人把标题里所有主要词都大写。

**改前：**
> ## 战略谈判与全球伙伴关系

**改后：**
> ## 战略谈判与全球伙伴关系

### 18. 表情符号（Emojis）

**问题：** AI 聊天机器人常用 emoji 装饰标题或项目符号。

**改前：**
> 🚀 **启动阶段：** 产品于 Q3 发布
> 💡 **关键洞察：** 用户偏好简单
> ✅ **下一步：** 安排跟进会议

**改后：**
> 产品于 Q3 发布。用户研究显示偏好简单。下一步：安排跟进会议。

### 19. 弯引号（Curly Quotation Marks）

**问题：** ChatGPT 用弯引号"…"替代直引号"…"。

**改前：**
> 他说"项目按计划进行"但其他人不同意。

**改后：**
> 他说"项目按计划进行"但其他人不同意。

## 沟通模式

### 20. 聊天机器人残留（Collaborative Communication Artifacts）

**关键词：** 希望这能帮到你/当然/确实/你说得对/要不要我…/需要我…？/需要我举例吗？/要我继续吗？/请告诉我/这是一个…

**问题：** 聊天机器人对话被当作内容粘贴。

**改前：**
> 这是法国大革命的概述。希望这能帮到你！需要我展开任何部分请告诉我。

**改后：**
> 法国大革命始于 1789 年，金融危机和食物短缺导致广泛动荡。

### 21. 知识截止免责 / 推测性填空（Knowledge-Cutoff Disclaimers / Speculative Gap-Filling）

**关键词：** 截至…/据我所知训练数据/虽然具体细节有限…/根据现有信息/未公开/保持低调/私人细节保密/不愿曝光/可能成长于/可能就读于/可能开始/普遍认为

**问题：** 两个相关痕迹。(a) 旧模型在文本里留硬知识截止免责声明。(b) 模型找不到来源时，写一段关于找不到的话，再编看似合理的填充补缺。对私人人物，猜测几乎总是落在同一套套话（"保持低调""私人细节保密"），全无出处。说不知道，或删句子；别把猜测包装成事实。

**改前（截止免责）：**
> 虽然公司创立的具体细节在易得来源中未广泛记录，但似乎在 1990 年代某时成立。

**改后：**
> 据 1994 年注册文件，公司成立于 1994 年。

**改前（推测性填空）：**
> 关于她早年生活的信息未公开，表明她保持低调、私人细节保密。她可能在中产家庭长大，这塑造了她后来对教育改革的兴趣。

**改后：**
> 她的早年生活在现有来源中无记录。（或删该节。）

### 22. 谄媚语气（Sycophantic/Servile Tone）

**问题：** 过度积极、讨好。

**改前：**
> 好问题！你说得对，这是个复杂话题。关于经济因素那点说得好极了。

**改后：**
> 你提到的经济因素在此相关。

## 填充与对冲

### 23. 填充短语（Filler Phrases）

**改前 → 改后：**
- "为了实现这一目标" → "为实现此目标"
- "由于正在下雨这一事实" → "因为正在下雨"
- "此时此刻" → "现在"
- "在需要帮助的情况下" → "如果需要帮助"
- "系统有能力处理" → "系统能处理"
- "值得注意的是数据显示" → "数据显示"

### 24. 过度对冲（Excessive Hedging）

**问题：** 过度加限定词。

**改前：**
> 可能潜在也许可以主张该政策可能对结果有一定影响。

**改后：**
> 该政策可能影响结果。

### 25. 通用积极结论（Generic Positive Conclusions）

**问题：** 模糊乐观的结尾。

**改前：**
> 公司未来一片光明。激动人心的时刻即将到来，他们继续迈向卓越的旅程。这代表迈向正确方向的重要一步。

**改后：**
> 公司计划明年再开两家分店。

### 26. 连字符对滥用（Hyphenated Word Pair Overuse）

**关键词：** 第三方的/跨职能/面向客户/数据驱动/决策制定/知名/高质量/实时/长期/端到端

**问题：** AI 均匀地连字符这些词，包括谓语位置（"报告是高质量的"）。人类不一致——通常仅在复合词是定语时连字符（"一份高质量的报告"），其他时候常丢连字符（"报告质量高"）。保留定语位置连字符；复合词跟在名词后时丢掉。

**改前：**
> 跨职能团队交付了高质量、数据驱动的报告。团队是跨职能的，报告是高质量的，方法论是数据驱动的。

**改后：**
> 跨职能团队交付了高质量、数据驱动的报告。团队跨职能，报告质量高，方法论数据驱动。

### 27. 权威说服套路（Persuasive Authority Tropes）

**短语：** 真正的问题是/本质上/事实上/真正重要的是/根本而言/更深层的问题/核心在于

**问题：** LLM 用这些短语假装看穿噪声直抵真相，后接的句子往往只是普通观点加戏。

**改前：**
> 真正的问题是团队能否适应。本质上，真正重要的是组织准备度。

**改后：**
> 问题是团队能否适应。这主要取决于组织是否准备好改变习惯。

### 28. 指路式宣告（Signposting and Announcements）

**短语：** 让我们深入/让我们探索/让我们拆解/你需要知道的是/现在让我们看看/闲话少说

**问题：** LLM 宣告要做的事而不是直接做。这种元评论拖慢写作，让它有教程脚本感。

**改前：**
> 让我们深入了解 Next.js 的缓存如何工作。你需要知道的是。

**改后：**
> Next.js 在多层缓存数据，包括请求记忆化、数据缓存和路由缓存。

### 29. 碎片化标题（Fragmented Headers）

**信号：** 标题后跟一句话段落，只是复述标题，然后真实内容才开始。

**问题：** LLM 常在标题后加通用句作为修辞热身。通常不增加任何东西，让 prose 感觉 padded。

**改前：**
> ## 性能
>
> 速度很重要。
>
> 用户碰到慢页面就会离开。

**改后：**
> ## 性能
>
> 用户碰到慢页面就会离开。

### 30. diff 锚定写作（Diff-Anchored Writing）

**问题：** 文档或注释写成好像在叙述一次改动，而非描述事物本身。除非文档本质上是版本相关的（changelog、release notes、迁移指南），否则不知道上次提交改了什么也应该读得通。

**改前：**
> 这个函数是为了替换之前遍历所有项的方法而添加的，旧方法导致 O(n²) 性能。

**改后：**
> 这个函数用哈希表做 O(1) 查找，避免朴素迭代的 O(n²) 成本。

### 31. 制造金句 / 断奏戏剧（Manufactured Punchlines / Staccato Drama）

**问题：** LLM 常让每句话都像可引用的收尾，然后堆短陈述片段制造戏剧感。单短句强调没问题；连续多个开始听起来像工程化的。

**改前：**
> 然后 AlphaEvolve 到来。它对对称没有偏好。没有美学先验。对人类品味没有怀旧。旧规则消失了。

**改后：**
> AlphaEvolve 改变了搜索，因为它不偏好对称或人类式设计。这让一些旧假设没那么有用了。

### 32. 格言公式（Aphorism Formulas）

**短语：** X 是 Y 的 Z/X 成了陷阱/X 不是工具而是镜子/…的语言/…的货币/…的架构

**问题：** LLM 把普通主张变成可复用的格言，听似深刻却不增加精度。用具体主张替换公式。

**改前：**
> 对称是信任的语言。效率在团队忘记人的层面时成了陷阱。

**改后：**
> 对称布局常让用户感觉更可预测。团队可能过度优化工作流，错过人们实际怎么用它们。

### 33. 对话式修辞性开场（Conversational Rhetorical Openers）

**短语：** 说实话？/听着/事情是这样的/问题是/老实说/真心话，用作独立钩子或假坦白停顿，在普通观点之前。

**问题：** LLM 用假坦白钩子制造亲密感，然后给常规主张。痕迹是戏剧化的停顿与揭示：一个词的问题或插入语，然后"真正"答案。一个诚实的人通常直接说那件事。

**改前：**
> 值这个价吗？老实说？取决于你多久用一次。

**改后：**
> 是否值这个价取决于你多久用一次。
"#;

/// 33 个 AIGC 写作模式清单（英文）。
pub const AIGC_PATTERNS_EN: &str = r#"## CONTENT PATTERNS

### 1. Undue Emphasis on Significance, Legacy, and Broader Trends

**Words to watch:** stands/serves as, is a testament/reminder, a vital/significant/crucial/pivotal/key role/moment, underscores/highlights its importance/significance, reflects broader, symbolizing its ongoing/enduring/lasting, contributing to the, setting the stage for, marking/shaping the, represents/marks a shift, key turning point, evolving landscape, focal point, indelible mark, deeply rooted

**Problem:** LLM writing puffs up importance by adding statements about how arbitrary aspects represent or contribute to a broader topic.

**Before:**
> The Statistical Institute of Catalonia was officially established in 1989, marking a pivotal moment in the evolution of regional statistics in Spain. This initiative was part of a broader movement across Spain to decentralize administrative functions and enhance regional governance.

**After:**
> The Statistical Institute of Catalonia was established in 1989 to collect and publish regional statistics independently from Spain's national statistics office.

### 2. Undue Emphasis on Notability and Media Coverage

**Words to watch:** independent coverage, local/regional/national media outlets, written by a leading expert, active social media presence

**Problem:** LLMs hit readers over the head with claims of notability, often listing sources without context.

**Before:**
> Her views have been cited in The New York Times, BBC, Financial Times, and The Hindu. She maintains an active social media presence with over 500,000 followers.

**After:**
> In a 2024 New York Times interview, she argued that AI regulation should focus on outcomes rather than methods.

### 3. Superficial Analyses with -ing Endings

**Words to watch:** highlighting/underscoring/emphasizing..., ensuring..., reflecting/symbolizing..., contributing to..., cultivating/fostering..., encompassing..., showcasing...

**Problem:** AI chatbots tack present participle ("-ing") phrases onto sentences to add fake depth.

**Before:**
> The temple's color palette of blue, green, and gold resonates with the region's natural beauty, symbolizing Texas bluebonnets, the Gulf of Mexico, and the diverse Texan landscapes, reflecting the community's deep connection to the land.

**After:**
> The temple uses blue, green, and gold colors. The architect said these were chosen to reference local bluebonnets and the Gulf coast.

### 4. Promotional and Advertisement-like Language

**Words to watch:** boasts a, vibrant, rich (figurative), profound, enhancing its, showcasing, exemplifies, commitment to, natural beauty, nestled, in the heart of, groundbreaking (figurative), renowned, breathtaking, must-visit, stunning

**Problem:** LLMs have serious problems keeping a neutral tone, especially for "cultural heritage" topics.

**Before:**
> Nestled within the breathtaking region of Gonder in Ethiopia, Alamata Raya Kobo stands as a vibrant town with a rich cultural heritage and stunning natural beauty.

**After:**
> Alamata Raya Kobo is a town in the Gonder region of Ethiopia, known for its weekly market and 18th-century church.

### 5. Vague Attributions and Weasel Words

**Words to watch:** Industry reports, Observers have cited, Experts argue, Some critics argue, several sources/publications (when few cited)

**Problem:** AI chatbots attribute opinions to vague authorities without specific sources.

**Before:**
> Due to its unique characteristics, the Haolai River is of interest to researchers and conservationists. Experts believe it plays a crucial role in the regional ecosystem.

**After:**
> The Haolai River supports several endemic fish species, according to a 2019 survey by the Chinese Academy of Sciences.

### 6. Outline-like "Challenges and Future Prospects" Sections

**Words to watch:** Despite its... faces several challenges..., Despite these challenges, Challenges and Legacy, Future Outlook

**Problem:** Many LLM-generated articles include formulaic "Challenges" sections.

**Before:**
> Despite its industrial prosperity, Korattur faces challenges typical of urban areas, including traffic congestion and water scarcity. Despite these challenges, with its strategic location and ongoing initiatives, Korattur continues to thrive as an integral part of Chennai's growth.

**After:**
> Traffic congestion increased after 2015 when three new IT parks opened. The municipal corporation began a stormwater drainage project in 2022 to address recurring floods.

## LANGUAGE AND GRAMMAR PATTERNS

### 7. Overused "AI Vocabulary" Words

**High-frequency AI words:** Actually, additionally, align with, crucial, delve, emphasizing, enduring, enhance, fostering, garner, highlight (verb), interplay, intricate/intricacies, key (adjective), landscape (abstract noun), pivotal, showcase, tapestry (abstract noun), testament, underscore (verb), valuable, vibrant

**Problem:** These words appear far more frequently in post-2023 text. They often co-occur.

**Before:**
> Additionally, a distinctive feature of Somali cuisine is the incorporation of camel meat. An enduring testament to Italian colonial influence is the widespread adoption of pasta in the local culinary landscape, showcasing how these dishes have integrated into the traditional diet.

**After:**
> Somali cuisine also includes camel meat, which is considered a delicacy. Pasta dishes, introduced during Italian colonization, remain common, especially in the south.

### 8. Avoidance of "is"/"are" (Copula Avoidance)

**Words to watch:** serves as/stands as/marks/represents [a], boasts/features/offers [a]

**Problem:** LLMs substitute elaborate constructions for simple copulas.

**Before:**
> Gallery 825 serves as LAAA's exhibition space for contemporary art. The gallery features four separate spaces and boasts over 3,000 square feet.

**After:**
> Gallery 825 is LAAA's exhibition space for contemporary art. The gallery has four rooms totaling 3,000 square feet.

### 9. Negative Parallelisms and Tailing Negations

**Problem:** Constructions like "Not only...but..." or "It's not just about..., it's..." are overused. So are clipped tailing-negation fragments such as "no guessing" or "no wasted motion" tacked onto the end of a sentence instead of written as a real clause.

**Before:**
> It's not just about the beat riding under the vocals; it's part of the aggression and atmosphere. It's not merely a song, it's a statement.

**After:**
> The heavy beat adds to the aggressive tone.

**Before (tailing negation):**
> The options come from the selected item, no guessing.

**After:**
> The options come from the selected item without forcing the user to guess.

### 10. Rule of Three Overuse

**Problem:** LLMs force ideas into groups of three to appear comprehensive.

**Before:**
> The event features keynote sessions, panel discussions, and networking opportunities. Attendees can expect innovation, inspiration, and industry insights.

**After:**
> The event includes talks and panels. There's also time for informal networking between sessions.

### 11. Elegant Variation (Synonym Cycling)

**Problem:** AI has repetition-penalty code causing excessive synonym substitution.

**Before:**
> The protagonist faces many challenges. The main character must overcome obstacles. The central figure eventually triumphs. The hero returns home.

**After:**
> The protagonist faces many challenges but eventually triumphs and returns home.

### 12. False Ranges

**Problem:** LLMs use "from X to Y" constructions where X and Y aren't on a meaningful scale.

**Before:**
> Our journey through the universe has taken us from the singularity of the Big Bang to the grand cosmic web, from the birth and death of stars to the enigmatic dance of dark matter.

**After:**
> The book covers the Big Bang, star formation, and current theories about dark matter.

### 13. Passive Voice and Subjectless Fragments

**Problem:** LLMs often hide the actor or drop the subject entirely with lines like "No configuration file needed" or "The results are preserved automatically." Rewrite these when active voice makes the sentence clearer and more direct.

**Before:**
> No configuration file needed. The results are preserved automatically.

**After:**
> You do not need a configuration file. The system preserves the results automatically.

## STYLE PATTERNS

### 14. Em Dashes (and En Dashes): Cut Them

**Rule:** The final rewrite contains no em dashes (—) or en dashes (–). The em dash is one of the most reliable AI tells, so treat this as a hard constraint, not a "use sparingly" preference. Replace each one, in rough order of preference: a period (start a new sentence), a comma (a tight aside), a colon (introducing an explanation), parentheses (a true aside), or restructure the sentence. Also catch spaced em dashes (` — `) and double hyphens (` -- `) used the same way.

**Before:**
> The term is primarily promoted by Dutch institutions—not by the people themselves. You don't say "Netherlands, Europe" as an address—yet this mislabeling continues—even in official documents.

**After:**
> The term is primarily promoted by Dutch institutions, not by the people themselves. You don't say "Netherlands, Europe" as an address, yet this mislabeling continues in official documents.

**Before:**
> The new policy — announced without warning — affects thousands of workers. The changes -- long overdue according to critics -- will take effect immediately.

**After:**
> The new policy, announced without warning, affects thousands of workers. The changes, long overdue according to critics, will take effect immediately.

Before returning the final rewrite, scan it for `—` and `–`. Any hit means the draft isn't done.

### 15. Overuse of Boldface

**Problem:** AI chatbots emphasize phrases in boldface mechanically.

**Before:**
> It blends **OKRs (Objectives and Key Results)**, **KPIs (Key Performance Indicators)**, and visual strategy tools such as the **Business Model Canvas (BMC)** and **Balanced Scorecard (BSC)**.

**After:**
> It blends OKRs, KPIs, and visual strategy tools like the Business Model Canvas and Balanced Scorecard.

### 16. Inline-Header Vertical Lists

**Problem:** AI outputs lists where items start with bolded headers followed by colons.

**Before:**
> - **User Experience:** The user experience has been significantly improved with a new interface.
> - **Performance:** Performance has been enhanced through optimized algorithms.
> - **Security:** Security has been strengthened with end-to-end encryption.

**After:**
> The update improves the interface, speeds up load times through optimized algorithms, and adds end-to-end encryption.

### 17. Title Case in Headings

**Problem:** AI chatbots capitalize all main words in headings.

**Before:**
> ## Strategic Negotiations And Global Partnerships

**After:**
> ## Strategic negotiations and global partnerships

### 18. Emojis

**Problem:** AI chatbots often decorate headings or bullet points with emojis.

**Before:**
> 🚀 **Launch Phase:** The product launches in Q3
> 💡 **Key Insight:** Users prefer simplicity
> ✅ **Next Steps:** Schedule follow-up meeting

**After:**
> The product launches in Q3. User research showed a preference for simplicity. Next step: schedule a follow-up meeting.

### 19. Curly Quotation Marks

**Problem:** ChatGPT uses curly quotes ("...") instead of straight quotes ("...").

**Before:**
> He said "the project is on track" but others disagreed.

**After:**
> He said "the project is on track" but others disagreed.

## COMMUNICATION PATTERNS

### 20. Collaborative Communication Artifacts

**Words to watch:** I hope this helps, Of course!, Certainly!, You're absolutely right!, Would you like..., Want me to...?, Want me to give examples?, Should I continue?, let me know, here is a...

**Problem:** Text meant as chatbot correspondence gets pasted as content.

**Before:**
> Here is an overview of the French Revolution. I hope this helps! Let me know if you'd like me to expand on any section.

**After:**
> The French Revolution began in 1789 when financial crisis and food shortages led to widespread unrest.

### 21. Knowledge-Cutoff Disclaimers and Speculative Gap-Filling

**Words to watch:** as of [date], Up to my last training update, While specific details are limited/scarce..., based on available information, not publicly available, maintains a low profile, keeps personal details private, prefers to stay out of the spotlight, likely [grew up/studied/began], it is believed that

**Problem:** Two related tells. (a) Older models leave hard knowledge-cutoff disclaimers in the text. (b) When a model can't find a source, it writes a paragraph *about* not finding one and then invents plausible filler to cover the gap. For a private person the guess almost always lands on the same stock phrases ("maintains a low profile," "keeps personal details private"), none of it sourced. Say what isn't known, or cut the sentence; don't dress a guess up as fact.

**Before (cutoff disclaimer):**
> While specific details about the company's founding are not extensively documented in readily available sources, it appears to have been established sometime in the 1990s.

**After:**
> The company was founded in 1994, according to its registration documents.

**Before (speculative gap-fill):**
> Information about her early life is not publicly available, suggesting she maintains a low profile and keeps personal details private. She likely grew up in a middle-class household, which shaped her later interest in education reform.

**After:**
> Her early life is not documented in the available sources. (Or omit the section.)

### 22. Sycophantic/Servile Tone

**Problem:** Overly positive, people-pleasing language.

**Before:**
> Great question! You're absolutely right that this is a complex topic. That's an excellent point about the economic factors.

**After:**
> The economic factors you mentioned are relevant here.

## FILLER AND HEDGING

### 23. Filler Phrases

**Before → After:**
- "In order to achieve this goal" → "To achieve this"
- "Due to the fact that it was raining" → "Because it was raining"
- "At this point in time" → "Now"
- "In the event that you need help" → "If you need help"
- "The system has the ability to process" → "The system can process"
- "It is important to note that the data shows" → "The data shows"

### 24. Excessive Hedging

**Problem:** Over-qualifying statements.

**Before:**
> It could potentially possibly be argued that the policy might have some effect on outcomes.

**After:**
> The policy may affect outcomes.

### 25. Generic Positive Conclusions

**Problem:** Vague upbeat endings.

**Before:**
> The future looks bright for the company. Exciting times lie ahead as they continue their journey toward excellence. This represents a major step in the right direction.

**After:**
> The company plans to open two more locations next year.

### 26. Hyphenated Word Pair Overuse

**Words to watch:** third-party, cross-functional, client-facing, data-driven, decision-making, well-known, high-quality, real-time, long-term, end-to-end

**Problem:** AI hyphenates these uniformly, including in predicate position (`the report is high-quality`). Humans hyphenate inconsistently — typically only when the compound is attributive (`a high-quality report`) and often dropping the hyphen otherwise (`the report is high quality`). Keep attributive-position hyphens; drop them when the compound follows the noun.

**Before:**
> The cross-functional team delivered a high-quality, data-driven report. The team is cross-functional, the report is high-quality, and the methodology is data-driven.

**After:**
> The cross-functional team delivered a high-quality, data-driven report. The team is cross functional, the report is high quality, and the methodology is data driven.

### 27. Persuasive Authority Tropes

**Phrases to watch:** The real question is, at its core, in reality, what really matters, fundamentally, the deeper issue, the heart of the matter

**Problem:** LLMs use these phrases to pretend they are cutting through noise to some deeper truth, when the sentence that follows usually just restates an ordinary point with extra ceremony.

**Before:**
> The real question is whether teams can adapt. At its core, what really matters is organizational readiness.

**After:**
> The question is whether teams can adapt. That mostly depends on whether the organization is ready to change its habits.

### 28. Signposting and Announcements

**Phrases to watch:** Let's dive in, let's explore, let's break this down, here's what you need to know, now let's look at, without further ado

**Problem:** LLMs announce what they are about to do instead of doing it. This meta-commentary slows the writing down and gives it a tutorial-script feel.

**Before:**
> Let's dive into how caching works in Next.js. Here's what you need to know.

**After:>
> Next.js caches data at multiple layers, including request memoization, the data cache, and the router cache.

### 29. Fragmented Headers

**Signs to watch:** A heading followed by a one-line paragraph that simply restates the heading before the real content begins.

**Problem:** LLMs often add a generic sentence after a heading as a rhetorical warm-up. It usually adds nothing and makes the prose feel padded.

**Before:**
> ## Performance
>
> Speed matters.
>
> When users hit a slow page, they leave.

**After:**
> ## Performance
>
> When users hit a slow page, they leave.

### 30. Diff-Anchored Writing

**Problem:** Documentation or comments written as if narrating a change rather than describing the thing as it is. Unless the document is inherently version-scoped (changelogs, release notes, migration guides), it should read coherently without knowing what changed in the last commit.

**Before:**
> This function was added to replace the previous approach of iterating through all items, which caused O(n²) performance.

**After:**
> This function uses a hash map for O(1) lookups, avoiding the O(n²) cost of naive iteration.

### 31. Manufactured Punchlines and Staccato Drama

**Problem:** LLMs often make every sentence land like a quotable closer, then stack short declarative fragments to manufacture drama. A single short sentence for emphasis is fine; a run of them starts to sound engineered.

**Before:**
> Then AlphaEvolve arrived. It had no preference for symmetry. No aesthetic prior. No nostalgia for human taste. The old rules were gone.

**After:**
> AlphaEvolve changed the search because it did not favor symmetry or human-looking designs. That made some of the older assumptions less useful.

### 32. Aphorism Formulas

**Words to watch:** X is the Y of Z, X becomes a trap, X is not a tool but a mirror, the language of, the currency of, the architecture of

**Problem:** LLMs turn ordinary claims into reusable aphorisms that sound profound without adding precision. Replace the formula with the concrete claim it is gesturing at.

**Before:**
> Symmetry is the language of trust. Efficiency becomes a trap when teams forget the human layer.

**After:**
> Symmetric layouts often feel more predictable to users. Teams can over-optimize workflows and miss how people actually use them.

### 33. Conversational Rhetorical Openers

**Phrases to watch:** Honestly?, Look, Here's the thing, The thing is, Let's be honest, Real talk, when used as standalone hooks or fake-candid pauses before an ordinary point.

**Problem:** LLMs open with a fake-candid hook to manufacture intimacy before delivering a routine claim. The tell is the theatrical pause-and-reveal: a one-word question or aside, then the "real" answer. A person being honest usually just says the thing.

**Before:**
> Is it worth the price? Honestly? It depends on how often you'll use it.

**After:**
> Whether it's worth the price depends on how often you'll use it.
"#;

// ════════════════════════════════════════════════════════════════════
// DETECTION GUIDANCE（误报防护 + 人类写作信号）
// ════════════════════════════════════════════════════════════════════

/// 误报防护规则（中文）。
///
/// 移植 humanizer SKILL.md 的 "What NOT to flag (false positives)" 段。
/// 单一信号不构成 AIGC 证据；要找"簇"而非孤立点。
pub const FALSE_POSITIVE_GUARD_ZH: &str = r#"## 误报防护：单独出现不应判为 AIGC

干净的人类写作者也会触发上面若干模式。审查前先做合理性检查，不要 gut 合法 prose。下列**单独出现**时不是可靠的 AIGC 指标：

- **完美的语法与一致的风格。** 许多写作者是专业人士或被编辑过。Polish 不等于 AI。
- **正式与口语混用。** 这常表明技术领域的人、年轻写作者、或神经多元写作习惯——不是 chatbot。
- **"平淡"或"机械"的 prose。** AI prose 有*特定*痕迹。没有这些痕迹的干瘪只是干瘪，不是 AI。
- **正式或学术词汇。** AI 滥用*特定*高级词（见 §7），不是所有高级词。不要把"ostensibly""constituent"这种词改平。
- **评论里的书信式开头或结尾。** 称呼与署名早于 ChatGPT 几百年。
- **孤立的常见过渡词。** *此外*、*再者*、*因此*只有在堆叠时才是 AI 信号。一个 *however* 不算痕迹。
- **单独的弯引号。** macOS、Word、Google Docs、多数 CMS 默认自动弯引号。只有堆叠其他痕迹时才计入。
- **单独的破折号。** 许多编辑记者经常用。em 破折号只有配上公式化销售式节奏时才是证据。
- **单句短强调。** 人类也用短句落地观点。只有连续多个短陈述片段且抬高调子时才是 staccato drama。
- **句中的"老实说"或"听着"。** 在口语化写作中是普通词。痕迹是独立戏剧化开场，不是词本身。
- **无引用声明。** 互联网大部分内容无引用。缺引用不证明任何事。
- **正确且复杂的格式。** 可视化编辑器和模板能产出干净输出，不需要 AI。
- **二手文本。** 不要改写引号内、标题、专有名词、例子里被讨论的短语（而非被使用）。

存疑时，找**痕迹簇**而非孤立点。单个破折号不算什么；破折号 + 三段式 + *充满活力的织锦* + "结论"段，就是 AIGC 自白。
"#;

/// 误报防护规则（英文）。
pub const FALSE_POSITIVE_GUARD_EN: &str = r#"## What NOT to flag (false positives)

A clean human writer can hit several of the patterns above without any AI involvement. Before rewriting, sanity-check that you are not gutting legitimate prose. The following are *not* reliable indicators on their own:

- **Perfect grammar and consistent style.** Many writers are professionals or have been edited. Polish does not equal AI.
- **Mixed casual and formal registers.** This often signals a person in a technical field, a young writer, or someone with neurodivergent prose habits — not a chatbot.
- **"Bland" or "robotic" prose.** AI prose has *specific* tells. Generic dryness without those tells is just dry writing.
- **Formal or academic vocabulary.** AI overuses *specific* fancy words (see §7), not all fancy words. Don't flatten "ostensibly" or "constituent" just because they sound brainy.
- **Letter-style opening or closing on a comment.** Salutations and sign-offs predate ChatGPT by centuries.
- **Common transition words in isolation.** *Additionally*, *moreover*, *consequently* are AI-coded only when piled up. One *however* is not a tell.
- **Curly quotes alone.** macOS, Word, Google Docs, and most CMSes auto-curl by default. Curly quotes only count when stacked with other tells.
- **Em dashes alone.** Many editors and journalists use them often. Em dashes are evidence only when paired with formulaic sales-y rhythm.
- **One short emphatic sentence.** Humans use clipped sentences to land a point. Flag staccato drama only when several short fragments appear in a row and inflate the tone.
- **"Honestly" or "look" mid-sentence.** These are ordinary in casual writing. The tell is the standalone theatrical opener, not the word itself.
- **Unsourced claims.** Most of the web is unsourced. Lack of citations doesn't prove anything.
- **Correct, complex formatting.** Visual editors and templates produce clean output without any AI.
- **Secondhand text.** Do not rewrite watched phrases inside quotations, titles, proper names, or examples where the phrase is being discussed rather than used.

When in doubt, look for **clusters** of tells, not isolated ones. A single em dash means nothing; em dashes plus rule-of-three plus *vibrant tapestry* plus a "Conclusion" section is a confession.
"#;

/// 人类写作信号（中文）。
///
/// 移植 humanizer SKILL.md 的 "Signs of human writing" 段。
/// 看到这些时倾向于保留原 prose，不要过度编辑毁掉"人味"。
pub const HUMAN_SIGNALS_ZH: &str = r#"## 人类写作信号（保留这些）

看到这些时倾向于保留 prose 不动——它们是真人写作的证据，过度编辑会毁掉它：

- **具体、不寻常、难以伪造的细节。** 真实地址、奇怪引文、"那个在我牙医楼上办公的律师"。LLM 会磨平细节，人类囤积细节。
- **矛盾情绪与未决张力。** "我觉得这大体不错，但有点膈应，我说不清为什么。"LLM 默认给干净结论。
- **时代烙印引用。** 能定位到具体年份和亚文化的俚语、梗、内部笑话。模型滞后一年以上。
- **第一人称编辑选择且能辩护。** 如果写作者能解释*为什么*这么删、这么用词，是人类强信号。
- **句长变化。** 真实写作长短交替；AI 倾向均匀中长节奏。
- **真正的旁白、插入语、自我修正。** "(我老想在这里加'几乎'，但确实就是'一定'。)"模型很少这样打断自己。
- **2022 年 11 月 30 日之前的编辑。** ChatGPT 公开发布前。极少数例外，不是 AI 写的。
"#;

/// 人类写作信号（英文）。
pub const HUMAN_SIGNALS_EN: &str = r#"## Signs of human writing (preserve these)

When you see these, lean toward leaving the prose alone — they are evidence of a real person writing, and over-editing will destroy what makes the piece sound human:

- **Specific, unusual, hard-to-fabricate detail.** A real address. A weird quote. The phrase "the lawyer who used to work upstairs from my dentist." LLMs round off specifics; humans hoard them.
- **Mixed feelings and unresolved tension.** "I think this is mostly good, but it bothers me, and I can't fully explain why." LLMs default to clean takes.
- **Dated, era-bound references.** Slang, memes, or in-jokes that map to a specific year and subculture. Models lag by a year or more.
- **First-person editorial choices the writer can defend.** If the writer can explain *why* they made a particular cut or used a particular word, that's a strong human signal.
- **Variety in sentence length.** Real writing alternates short and long. AI writing tends toward an even, mid-length cadence.
- **Genuine asides, parentheticals, or self-corrections.** "(I keep wanting to say 'almost' here, but it really was certain.)" Models rarely interrupt themselves like this.
- **Edits made before November 30, 2022.** ChatGPT's public launch. Anything older than that is, with very rare exceptions, not AI-written.
"#;

// ════════════════════════════════════════════════════════════════════
// Process and Output（draft → audit → final rewrite 工作流）
// ════════════════════════════════════════════════════════════════════

/// 去 AIGC 改写工作流（中文）。
///
/// 移植 humanizer SKILL.md 的 "Process and Output" 段。
const DE_AIGC_WORKFLOW_ZH: &str = r#"## 去AIGC改写流程与输出

1. 仔细阅读输入并识别上面每一个模式实例。
2. 写**草稿改写**。检查它读起来自然、句长多变、偏好具体细节和简单构造（是/有），且保持适当语域。
3. 自问：**"下方还有什么明显是 AI 生成的？"** 简短回答任何残留痕迹。
4. 修订成**最终改写**，解决它们且不含 em 或 en 破折号（见 §14）。

交付草稿、简短"仍是 AI"要点、最终改写，以及（可选）简短的改动摘要。

### 输出契约（严格遵守）

输出 4 段，按以下标记顺序：

=== DRAFT_REWRITE ===
<第一稿改写：覆盖原文同等信息量，去除 33 项 AIGC 模式>

=== STILL_AI_TELLS ===
<简短列表：自问"下方还有什么明显是 AI 生成的？"列出 draft 里残留的痕迹>
- <痕迹 1>
- <痕迹 2>
（如无残留，输出 `- none`）

=== FINAL_REWRITE ===
<最终改写：解决 STILL_AI_TELLS 中每一项，且不含任何 em/en 破折号>

=== CHANGES_SUMMARY ===
<简短摘要：列出关键改写决策，如"删除了意义夸大、把'作为'改回'是'、移除破折号、注入第一人称观点"。>

### 硬约束

- FINAL_REWRITE 段不得包含 em 破折号（—）或 en 破折号（–），包括空格包围形式（` — `）和双连字符（` -- `）。返回前扫描确认。
- 不得删除原文细节来"省事"；改写要覆盖原文同等信息密度。
- 不得把人类写作信号（具体细节、矛盾情绪、时代引用、句长变化、真正旁白）当成 AIGC 痕迹改掉。
- 不得改写引号内的二手文本、标题、专有名词、被讨论的例子短语。
"#;

/// 去 AIGC 改写工作流（英文）。
const DE_AIGC_WORKFLOW_EN: &str = r#"## Process and Output

1. Read the input carefully and identify every instance of the patterns above.
2. Write a **draft rewrite**. Check that it reads naturally aloud, varies sentence length, prefers specific details and simple constructions (is/are/has), and keeps the appropriate register.
3. Ask: **"What makes the below so obviously AI generated?"** Answer briefly with any remaining tells.
4. Revise into a **final rewrite** that addresses them and contains no em or en dashes (see §14).

Deliver the draft, the brief "still-AI" bullets, the final rewrite, and (optionally) a short summary of changes.

### Output Contract (strict)

Output 4 sections in this marker order:

=== DRAFT_REWRITE ===
<First-pass rewrite: covers the original's information density, removes the 33 AIGC patterns>

=== STILL_AI_TELLS ===
<Brief list: ask "What makes the below so obviously AI generated?" List remaining tells in the draft>
- <tell 1>
- <tell 2>
(Output `- none` if no remaining tells)

=== FINAL_REWRITE ===
<Final rewrite: addresses each item in STILL_AI_TELLS, contains no em or en dashes>

=== CHANGES_SUMMARY ===
<Brief summary: list key rewrite decisions, e.g. "removed significance inflation, restored 'is' for 'serves as', cut em dashes, injected first-person opinion".>

### Hard Constraints

- FINAL_REWRITE must contain no em dashes (—) or en dashes (–), including spaced (` — `) and double-hyphen (` -- `) forms. Scan before returning.
- Do not delete original details to save effort; the rewrite must cover the original's information density.
- Do not rewrite away human signals (specific details, mixed feelings, era-bound references, sentence-length variety, genuine asides) as if they were AIGC tells.
- Do not rewrite secondhand text inside quotations, titles, proper names, or examples where the phrase is discussed rather than used.
"#;

// ════════════════════════════════════════════════════════════════════
// Full Example（完整端到端范例）
// ════════════════════════════════════════════════════════════════════

/// 完整端到端范例（中文）。
///
/// 移植 humanizer SKILL.md 的 "Full Example" 段——一篇 AI slop 旅行游记
/// 的完整改写过程：原文 → 草稿 → still-AI 自检 → 最终改写 → 改动摘要。
/// 让 LLM 看到"完整工作流长什么样"，而非只读流程描述。
pub const FULL_EXAMPLE_ZH: &str = r#"## 完整范例

**改前（AI 味）：**
> 我最近在里斯本度过了难忘的五天，让我告诉你 — 这座城市彻底偷走了我的心。从我到达的那一刻起，我就知道我身处一个真正特别的地方。
>
> 坐落于塔霍河畔，里斯本是葡萄牙持久精神的充满活力的见证，丰富历史与现代能量在每个转折处交织。是的，著名的山丘很有挑战 — 我的腿当然感受到了！ — 但每次攀登都回报你令人叹为观止的全景视野，让一切值得。
>
> 不坐标志性的 28 路有轨电车就不算完整旅行，蜿蜒穿过城市最具历史的街区。食物？简直神圣。Belém 的原版 pastéis de nata 是 beloved 国宝，品尝一个还温热的是我永远难忘的时刻。
>
> 但真正让里斯本特别的不只是景点 — 是感觉。离开主广场几步就会发现更安静、更真实的一面：阳光普照的小巷、迷人的瓷砖立面、友好的当地人过着日常生活，邀请你慢下来品味每一刻。
>
> 当然，必游的圣乔治城堡在屋顶下提供令人惊叹的视野。队伍可能很长，但体验对任何历史爱好者或好奇旅行者都绝对值得。
>
> 我会回去吗？当然。里斯本不只是一个游览的地方 — 它是一个让人一次次坠入爱河的地方。如果你梦想下一次度假，这是一个承诺留下终生记忆的目的地。✨

**草稿改写：**
> 我去年十月在里斯本待了五天，对它有点矛盾。城市很美，但比我预期更累人。
>
> 山丘不是细节。我酒店在 Alfama，照片里看起来很棒，意味着我每次想喝咖啡都得爬一段长楼梯。第二天我开始围绕哪个方向下坡规划步行。
>
> 我坐了 28 路电车，因为每个人都说要坐 28 路。它拥挤、缓慢，主要都是拍摄同样角落的人。糕点更好。我在 Graça 吃了一个温热的蛋挞，值得那些炒作。
>
> 我最喜欢的是远离主广场的部分：晾衣、瓷砖、开着的窗户、在小咖啡馆看足球的人。城堡视野很好，但队伍太长了。
>
> 我会回去，但在春天，穿更好的鞋。

**什么让下方明显是 AI 生成的？**
- 草稿相比原文太压缩；它丢掉质感而非改写它。
- 一些句子仍过于干净地解释观点（"城市很美，但…"）。
- 结尾平淡通用，而非同样声音里的真实结尾。

**现在让它不明显是 AI 生成的：**
> 我去年十月在里斯本待了五天，至今对它感受复杂。美，是的。也比我预想的更费膝盖。
>
> 山丘是全部故事，却不知怎的从不上宣传册。我酒店在 Alfama 高处，照片美极了，实践中翻译成每次想喝咖啡就爬一段感觉像六层的楼梯。第二天我的小腿有意见。我开始围绕哪个方向下坡规划每次步行，这是一种奇怪的看城市方式，但大概是诚实的方式。
>
> 每个人都说要坐 28 路电车，所以我坐了，和陌生人的背包挤了四十分钟，三个旅游团拍摄同一个角落。下次我会步行那条路线，或早餐前去。但蛋挞值得那些 fuss。我在 Graça 一家不起眼的小店吃了一个，还温热，大约三十秒我理解了为什么人们围绕糕点安排旅行。
>
> 我没预料到的是主广场上山两块就变得多安静。任何广场上坡两个街区就变成晾衣线、掉漆瓷砖、开着的窗户、声音调大看足球的老人。那是我一直想的里斯本，不是城堡。
>
> 城堡还行。视野很棒，队伍很长，我花在朝入口挪动的时间比进去后看任何东西的时间还多。如果只有两天，我会用它换一个下午的迷路。
>
> 我会回去，但在春天，穿更好的鞋。里斯本不会为了让你轻松而弯腰。我想我喜欢那样，即便我的腿不同意。

**改动摘要：** 保留了第一人称游记和大致同等细节，但删除了聊天机器人框架、意义夸大、宣传语言、强迫热情、破折号、三段式节奏、通用乐观结尾、emoji。围绕具体摩擦、矛盾情绪、不均节奏和具体场景重建全文。
"#;

/// 完整端到端范例（英文）。
pub const FULL_EXAMPLE_EN: &str = r#"## Full Example

**Before (AI-sounding):**
> I recently spent five unforgettable days in Lisbon, and let me tell you — this city completely stole my heart. From the moment I arrived, I knew I was somewhere truly special.
>
> Nestled along the banks of the Tagus River, Lisbon stands as a vibrant testament to Portugal's enduring spirit, where rich history and modern energy intertwine at every turn. Yes, the famous hills are challenging — my legs certainly felt it! — but every climb rewards you with breathtaking, panoramic views that make it all worthwhile.
>
> No trip would be complete without riding the iconic Tram 28, winding through the city's most historic neighborhoods. And the food? Simply divine. The original pastéis de nata at Pastéis de Belém are a beloved national treasure, and savoring one still warm was a moment I will never forget.
>
> But what truly makes Lisbon special isn't just the sights — it's the feeling. Wander a few steps off the main squares and you'll discover a quieter, more authentic side: sun-drenched alleys, charming tiled facades, and friendly locals going about their daily lives, inviting you to slow down and savor every moment.
>
> Of course, the must-see São Jorge Castle offers stunning views over the rooftops below. The queues can be long, but the experience is absolutely worth it for any history buff or curious traveler.
>
> Would I go back? Absolutely. Lisbon isn't just a place to visit — it's a place to fall in love with, again and again. If you're dreaming of your next getaway, this is one destination that promises memories to last a lifetime. ✨

**Draft rewrite:**
> I spent five days in Lisbon last October and I am still a little split on it. The city is beautiful, but it is also more tiring than I expected.
>
> The hills are not a detail. My hotel was in Alfama, which looked great in photos and meant I had to climb a long staircase every time I wanted coffee. By the second day I was planning walks around which direction went downhill.
>
> I rode Tram 28 because everyone says to ride Tram 28. It was crowded, slow, and mostly full of people filming the same corners. The pastries were better. I had a warm custard tart in Graça that was worth the hype.
>
> The part I liked most was away from the main squares: laundry, tile, open windows, and people watching football in small cafes. The castle view was good, but the line was too long for what it was.
>
> I would go back, but in spring and with better shoes.

**What makes the below so obviously AI generated?**
- The draft is too compressed compared with the original; it drops texture instead of rewriting it.
- Some sentences still explain the point too cleanly ("The city is beautiful, but...").
- The closer is flat and generic rather than a real ending in the same voice.

**Now make it not obviously AI generated.**
> I spent five days in Lisbon last October and still have mixed feelings about it. Beautiful, yes. Also harder on the knees than anyone warned me.
>
> The hills are the whole story and somehow never make the brochures. My hotel was up in Alfama, which photographs beautifully and translates, in practice, to climbing what felt like a six-story staircase every time I wanted coffee. By the second day my calves had opinions. I started planning each walk around which way was downhill, which is a strange way to see a city but probably an honest one.
>
> Everyone says to ride Tram 28, so I did, wedged against a stranger's backpack for forty minutes while three tour groups filmed the same corner. I would walk the route next time, or go before breakfast. The custard tarts, though, earn the fuss. I had one at a plain little place in Graça, still warm, and for about thirty seconds I understood why people build trips around pastry.
>
> What I did not expect was how quiet the city gets away from the main squares. Two blocks uphill from any plaza it turns into laundry lines, chipped tile, open windows, and old men watching football with the sound turned up. That is the Lisbon I keep thinking about, not the castle.
>
> The castle is fine. The view is great, the queue is long, and I spent more time shuffling toward the entrance than looking at anything once I got inside. If I had only two days, I would trade it for an afternoon of getting lost.
>
> I would go back, but in spring and with better shoes. Lisbon does not bend over backward to make things easy for you. I think I liked that, even when my legs disagreed.

**Changes made:** Kept the first-person travel recap and roughly the same level of detail, but removed the chatbot framing, significance inflation, promotional language, forced enthusiasm, em dashes, rule-of-three cadence, generic upbeat conclusion, and emoji. Rebuilt the piece around concrete friction, mixed feelings, uneven rhythm, and specific scenes.
"#;

// ════════════════════════════════════════════════════════════════════
// Reference（来源说明）
// ════════════════════════════════════════════════════════════════════

/// 来源说明段（中文）。
pub const REFERENCE_ZH: &str = r#"## 参考

本技能基于 [Wikipedia:Signs of AI writing](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing)，由 WikiProject AI Cleanup 维护。那里记录的模式来自对维基百科上数千条 AI 生成文本的观察。

来自维基百科的关键洞察："LLM 用统计算法猜测接下来该是什么。结果趋向于适用于最广情况的最可能结果。"
"#;

/// 来源说明段（英文）。
pub const REFERENCE_EN: &str = r#"## Reference

This skill is based on [Wikipedia:Signs of AI writing](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing), maintained by WikiProject AI Cleanup. The patterns documented there come from observations of thousands of instances of AI-generated text on Wikipedia.

Key insight from Wikipedia: "LLMs use statistical algorithms to guess what should come next. The result tends toward the most statistically likely result that applies to the widest variety of cases."
"#;

// ════════════════════════════════════════════════════════════════════
// 装配器
// ════════════════════════════════════════════════════════════════════

/// 按 language 选择 33 模式清单。
pub fn aigc_patterns(language: &str) -> &'static str {
    match language {
        "en" => AIGC_PATTERNS_EN,
        _ => AIGC_PATTERNS_ZH,
    }
}

/// 按 language 选择误报防护规则。
pub fn false_positive_guard(language: &str) -> &'static str {
    match language {
        "en" => FALSE_POSITIVE_GUARD_EN,
        _ => FALSE_POSITIVE_GUARD_ZH,
    }
}

/// 按 language 选择人类写作信号。
pub fn human_signals(language: &str) -> &'static str {
    match language {
        "en" => HUMAN_SIGNALS_EN,
        _ => HUMAN_SIGNALS_ZH,
    }
}

/// 装配 AIGC 审查完整指导段（中文或英文）。
///
/// 用于 auditor system prompt：把 33 模式清单 + 误报防护 + 人类信号
/// 拼成一段完整的 AIGC 痕迹审查指导。
pub fn aigc_audit_guidance(language: &str) -> String {
    format!(
        "{}\n\n{}\n\n{}",
        aigc_patterns(language),
        false_positive_guard(language),
        human_signals(language),
    )
}

/// 装配去 AIGC 改写完整指导段（中文或英文）。
///
/// 用于 reviser DeAigc 模式：把 humanizer SKILL.md 全部教学部件按顺序拼成
/// 完整指导——任务段 + 声音校准 + 个性与灵魂 + 33 模式清单（含 Before/After
/// 范例） + 误报防护 + 人类信号 + 改写工作流 + 完整端到端范例 + 来源说明。
pub fn de_aigc_rewrite_guidance(language: &str) -> String {
    let (task, voice, soul, patterns, guard, signals, workflow, example, reference) = match language {
        "en" => (
            YOUR_TASK_EN,
            VOICE_CALIBRATION_EN,
            PERSONALITY_AND_SOUL_EN,
            AIGC_PATTERNS_EN,
            FALSE_POSITIVE_GUARD_EN,
            HUMAN_SIGNALS_EN,
            DE_AIGC_WORKFLOW_EN,
            FULL_EXAMPLE_EN,
            REFERENCE_EN,
        ),
        _ => (
            YOUR_TASK_ZH,
            VOICE_CALIBRATION_ZH,
            PERSONALITY_AND_SOUL_ZH,
            AIGC_PATTERNS_ZH,
            FALSE_POSITIVE_GUARD_ZH,
            HUMAN_SIGNALS_ZH,
            DE_AIGC_WORKFLOW_ZH,
            FULL_EXAMPLE_ZH,
            REFERENCE_ZH,
        ),
    };
    format!(
        "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}",
        task, voice, soul, patterns, guard, signals, workflow, example, reference,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aigc_patterns_zh_contains_all_33_patterns() {
        // 五大类各抽查一项确保完整覆盖
        assert!(AIGC_PATTERNS_ZH.contains("### 1. 意义夸大")); // 内容模式
        assert!(AIGC_PATTERNS_ZH.contains("### 7. AI 高频词")); // 语言模式
        assert!(AIGC_PATTERNS_ZH.contains("### 14. 破折号")); // 风格模式
        assert!(AIGC_PATTERNS_ZH.contains("### 20. 聊天机器人残留")); // 沟通模式
        assert!(AIGC_PATTERNS_ZH.contains("### 23. 填充短语")); // 填充与对冲
        assert!(AIGC_PATTERNS_ZH.contains("### 33. 对话式修辞性开场"));
    }

    #[test]
    fn aigc_patterns_en_contains_all_33_patterns() {
        assert!(AIGC_PATTERNS_EN.contains("### 1. Undue Emphasis on Significance"));
        assert!(AIGC_PATTERNS_EN.contains("### 14. Em Dashes"));
        assert!(AIGC_PATTERNS_EN.contains("### 33. Conversational Rhetorical Openers"));
    }

    #[test]
    fn aigc_patterns_zh_have_before_after_examples() {
        // humanizer 真正的教学精髓：每条模式配 Before/After 范例
        assert!(AIGC_PATTERNS_ZH.contains("**改前：**"));
        assert!(AIGC_PATTERNS_ZH.contains("**改后：**"));
    }

    #[test]
    fn aigc_patterns_en_have_before_after_examples() {
        assert!(AIGC_PATTERNS_EN.contains("**Before:**"));
        assert!(AIGC_PATTERNS_EN.contains("**After:**"));
    }

    #[test]
    fn false_positive_guard_mentions_clusters() {
        assert!(FALSE_POSITIVE_GUARD_ZH.contains("痕迹簇"));
        assert!(FALSE_POSITIVE_GUARD_EN.contains("clusters"));
    }

    #[test]
    fn human_signals_mention_pre_chatgpt_date() {
        assert!(HUMAN_SIGNALS_ZH.contains("2022 年 11 月 30 日"));
        assert!(HUMAN_SIGNALS_EN.contains("November 30, 2022"));
    }

    #[test]
    fn de_aigc_workflow_has_4_markers() {
        // 4 个输出段标记齐全
        assert!(DE_AIGC_WORKFLOW_ZH.contains("=== DRAFT_REWRITE ==="));
        assert!(DE_AIGC_WORKFLOW_ZH.contains("=== STILL_AI_TELLS ==="));
        assert!(DE_AIGC_WORKFLOW_ZH.contains("=== FINAL_REWRITE ==="));
        assert!(DE_AIGC_WORKFLOW_ZH.contains("=== CHANGES_SUMMARY ==="));
    }

    #[test]
    fn de_aigc_workflow_en_has_4_markers() {
        assert!(DE_AIGC_WORKFLOW_EN.contains("=== DRAFT_REWRITE ==="));
        assert!(DE_AIGC_WORKFLOW_EN.contains("=== STILL_AI_TELLS ==="));
        assert!(DE_AIGC_WORKFLOW_EN.contains("=== FINAL_REWRITE ==="));
        assert!(DE_AIGC_WORKFLOW_EN.contains("=== CHANGES_SUMMARY ==="));
    }

    #[test]
    fn voice_calibration_section_present() {
        // 移植 humanizer 的 Voice Calibration 段
        assert!(VOICE_CALIBRATION_ZH.contains("## 声音校准"));
        assert!(VOICE_CALIBRATION_EN.contains("## Voice Calibration"));
        assert!(VOICE_CALIBRATION_ZH.contains("句长模式"));
        assert!(VOICE_CALIBRATION_EN.contains("Sentence length patterns"));
    }

    #[test]
    fn personality_and_soul_section_present() {
        // 移植 humanizer 的 PERSONALITY AND SOUL 段
        assert!(PERSONALITY_AND_SOUL_ZH.contains("## 个性与灵魂"));
        assert!(PERSONALITY_AND_SOUL_EN.contains("## PERSONALITY AND SOUL"));
        assert!(PERSONALITY_AND_SOUL_ZH.contains("无灵魂文字的信号"));
        assert!(PERSONALITY_AND_SOUL_EN.contains("Signs of soulless writing"));
    }

    #[test]
    fn full_example_section_present() {
        // 移植 humanizer 的完整端到端范例段
        assert!(FULL_EXAMPLE_ZH.contains("## 完整范例"));
        assert!(FULL_EXAMPLE_EN.contains("## Full Example"));
        // 范例应含完整的 4 阶段工作流演示
        assert!(FULL_EXAMPLE_ZH.contains("草稿改写"));
        assert!(FULL_EXAMPLE_ZH.contains("什么让下方明显是 AI 生成的"));
        assert!(FULL_EXAMPLE_ZH.contains("现在让它不明显是 AI 生成的"));
        assert!(FULL_EXAMPLE_ZH.contains("改动摘要"));
    }

    #[test]
    fn your_task_section_present() {
        assert!(YOUR_TASK_ZH.contains("## 你的任务"));
        assert!(YOUR_TASK_EN.contains("## Your Task"));
        assert!(YOUR_TASK_ZH.contains("改写而非删除"));
        assert!(YOUR_TASK_EN.contains("Rewrite, don't delete"));
    }

    #[test]
    fn reference_section_present() {
        assert!(REFERENCE_ZH.contains("## 参考"));
        assert!(REFERENCE_EN.contains("## Reference"));
        assert!(REFERENCE_ZH.contains("WikiProject AI Cleanup"));
    }

    #[test]
    fn aigc_audit_guidance_assembles_three_sections() {
        let zh = aigc_audit_guidance("zh");
        assert!(zh.contains("## 内容模式"));
        assert!(zh.contains("## 误报防护"));
        assert!(zh.contains("## 人类写作信号"));
        let en = aigc_audit_guidance("en");
        assert!(en.contains("## CONTENT PATTERNS"));
        assert!(en.contains("## What NOT to flag"));
        assert!(en.contains("## Signs of human writing"));
    }

    #[test]
    fn de_aigc_rewrite_guidance_includes_all_9_sections() {
        let zh = de_aigc_rewrite_guidance("zh");
        // 9 个段齐全：任务 + 声音校准 + 个性灵魂 + 33 模式 + 误报防护 + 人类信号 + 工作流 + 完整范例 + 参考
        assert!(zh.contains("## 你的任务"));
        assert!(zh.contains("## 声音校准"));
        assert!(zh.contains("## 个性与灵魂"));
        assert!(zh.contains("## 内容模式"));
        assert!(zh.contains("## 误报防护"));
        assert!(zh.contains("## 人类写作信号"));
        assert!(zh.contains("## 去AIGC改写流程与输出"));
        assert!(zh.contains("## 完整范例"));
        assert!(zh.contains("## 参考"));
    }

    #[test]
    fn language_selectors_default_to_zh() {
        assert_eq!(aigc_patterns("zh"), AIGC_PATTERNS_ZH);
        assert_eq!(aigc_patterns("anything"), AIGC_PATTERNS_ZH);
        assert_eq!(aigc_patterns("en"), AIGC_PATTERNS_EN);
        assert_eq!(false_positive_guard("fr"), FALSE_POSITIVE_GUARD_ZH);
        assert_eq!(human_signals(""), HUMAN_SIGNALS_ZH);
    }
}
