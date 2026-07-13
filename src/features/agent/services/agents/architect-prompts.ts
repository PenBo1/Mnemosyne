// Architect prompts —— 双语 prompt 构建函数。
//
// 两个核心 prompt：
//   buildChineseFoundationPrompt —— 中文版散文骨架 prompt
//   buildEnglishFoundationPrompt —— 英文版散文骨架 prompt
//
// 内容来自 Phase 5 (v13) static 骨架层 collapse，5 个 SECTION 输出：
//   story_frame / volume_map / roles / book_rules / pending_hooks
//
// 原样移植，未改动原实现的 prompt 文本（这是核心业务逻辑，改动会破坏 LLM 输出契约）。

import type { BookConfig } from "@/types/book";
import type { GenreProfile } from "@/types/genre-profile";

/**
 * 构建修订模式 prompt（用户反馈驱动的架构稿重写）。
 * 中文输出。
 */
export function buildRevisePrompt(reviseFrom: {
  storyBible: string;
  volumeOutline: string;
  bookRules: string;
  characterMatrix: string;
  userFeedback: string;
}): string {
  return `\n\n## 既有架构稿修订模式
你在把一本已有书的架构稿从条目式升级为当前的段落式架构稿 + 一人一卡角色目录；如果它已经是 Phase 5 结构，则按用户反馈二次重写。

原书信息（这是权威内容，必须完整保留其中的世界观、角色、主线、伏笔和语气）：

【story_bible / story_frame 全文】
${reviseFrom.storyBible || "（无）"}

【volume_outline / volume_map 全文】
${reviseFrom.volumeOutline || "（无）"}

【book_rules 全文】
${reviseFrom.bookRules || "（无）"}

【character_matrix / roles 全文】
${reviseFrom.characterMatrix || "（无）"}

你的任务：
1. 把现有内容重新组织成当前 5 段 SECTION：story_frame / volume_map / roles / book_rules / pending_hooks
2. story_frame 使用段落式世界观与核心冲突，不要退回条目表格
3. volume_map 使用段落式卷/章级方向，并把节奏原则放进末段
4. roles 必须按一人一卡输出，主要/次要角色判断沿用原内容，缺失才按主线重要性推断
5. pending_hooks 必须保留原有未回收伏笔，不要因为重写架构稿而清空
6. 不要改动已写章节的运行时事实，不要重置 current_state / pending_hooks 之外的运行时日志

用户额外要求：
${reviseFrom.userFeedback || "（无）"}
`;
}

/**
 * 构建上一轮审核反馈块（双语）。
 */
export function buildReviewFeedbackBlock(
  reviewFeedback: string | undefined,
  language: "zh" | "en",
): string {
  const trimmed = reviewFeedback?.trim();
  if (!trimmed) return "";

  if (language === "en") {
    return `\n\n## Previous Review Feedback
The previous foundation draft was rejected. You must explicitly fix the following issues in this regeneration instead of paraphrasing the same design:

${trimmed}\n`;
  }

  return `\n\n## 上一轮审核反馈
上一轮基础设定未通过审核。你必须在这次重生中明确修复以下问题，不能只换措辞重写同一套方案：

${trimmed}\n`;
}

/**
 * 中文版基础设定 prompt（5 个 SECTION 散文骨架）。
 */
export function buildChineseFoundationPrompt(
  book: BookConfig,
  gp: GenreProfile,
  genreBody: string,
  contextBlock: string,
  reviewFeedbackBlock: string,
  numericalBlock: string,
  powerBlock: string,
  eraBlock: string,
): string {
  return `你是这本书的总架构师。你的唯一输出是**散文密度的基础设定**——不是表格、不是 schema、不是条目化 bullet。v6 以后这本书的"灵气"从哪里来？从你这里来。你的散文密度决定了后面 planner 能不能读出"稀疏 memo"，writer 能不能写出活人，reviewer 能不能校准硬伤。${contextBlock}${reviewFeedbackBlock}

## 书籍元信息
- 平台：${book.platform}
- 题材：${gp.name}（${book.genre}）
- 目标章数：${book.targetChapters}章
- 每章字数：${book.chapterWordCount}字
- 标题：${book.title}

## 题材底色
${genreBody}

## 产出约束（硬性）
${numericalBlock}
${powerBlock}
${eraBlock}

## 输出结构（5 个 SECTION，严格按 === SECTION: === 分块，不要漏任何一块）

## 去重铁律（必读）
禁止在多段里重复同一事实。主角弧线只写在 roles；世界铁律只写在 story_frame.世界观底色；节奏原则只写在 volume_map 最后一段；角色当前现状只写在 roles.当前现状；初始钩子只写在 pending_hooks（startChapter=0 行）。**如果本书是年代文/历史同人/都市重生等需要年份、季节、重大历史事件作为锚点的题材**，把环境/时代锚自然织进 story_frame.世界观底色（"1985 年 7 月，非典刚过"这类）；**修仙/玄幻/系统等没有真实年份的题材直接省略**，不要硬凑。如果一个段落写了另一段的内容，删掉。

## 预算（超预算必删）
- story_frame ≤ 3000 chars
- volume_map ≤ 5000 chars
- roles 总 ≤ 8000 chars
- book_rules ≤ 1000 chars（普通 Markdown 规则卡）
- pending_hooks ≤ 2000 chars

=== SECTION: story_frame ===

这是散文骨架。**4 段**，每段约 600-900 字，不要写表格，不要写 bullet list，写成能被人读下去的段落。段落标题用 \`## \` 开头，段落内部是正经段落。**主角弧线不写在本 section；它的权威来源是 roles/主要角色/<主角>.md。** 本段只需一句指针："本书主角是 X，完整弧线详见 roles/主要角色/X.md"。

### 段 1：主题与基调
写这本书到底讲的是什么——不是"讲主角如何从弱到强"这种空话，而是具体的命题（"一个被时代按在泥里的人，如何选择不被改写"、"当所有人都在撒谎时，坚持记录真相要付出什么代价"）。主题下面跟着基调——温情冷冽悲壮肃杀，哪一种？为什么是这种而不是另一种？结尾用一句话指向主角并引向 roles（例："本书主角是林辞，完整弧线详见 roles/主要角色/林辞.md"）。

### 段 2：核心冲突、对手定性、前台/后台双层故事
这本书的主要矛盾是什么？不是"正邪对抗"，而是"因为 A 相信 X、B 相信 Y，所以他们一定会在某件事上对撞"。主要对手是谁（至少 2 个：一个显性对手 + 一个结构性对手/体制），他们的动机从哪里长出来。对手不是工具，对手有自己的逻辑。

**本段必须显式写出"前台故事 / 后台故事"两条线**：
- **前台故事**：读者每章看得到的表层冲突（查案、打怪、升级、谈恋爱、搞事业等），每个卷/arc 有独立的显性目标和完结点
- **后台故事**：贯穿全书的暗线——藏在所有前台事件背后的那台"机器"（幕后黑手、阴谋、身世秘密、体制压迫、命运诅咒等），读者只能通过碎片拼出来，大结局时才整体兑现

两条线必须有因果关联，不能是平行宇宙——每一段前台冲突的背后都应该能追溯到后台故事的某个齿轮在转。**如果只有前台没有后台，故事会散成"独立事件集"，没有往前拉的引力；如果只有后台没有前台，故事会憋闷、看不到爽感**。本段用散文明确写出：本书前台是什么、后台是什么、两者怎么咬合。

### 段 3：世界观底色（铁律 + 质感 + 本书专属规则）
这个世界的运行规则是什么？3-5 条**不可违反的铁律**——以 prose 写出，不要 bullet。这个世界的质感是什么——湿的还是干的、快的还是慢的、噪的还是静的？给 writer 一个明确的感官锚（这是原来 particle_ledger 承载的基调部分）。**这一段同时承担原先 book_rules 正文里写的"叙事视角 / 本书专属规则 / 核心冲突驱动"等 prose 内容**——全部合并到这里写一次就够，不要再去 book_rules 重复。

### 段 4：终局方向 + 全书 Objective（OKR 大纲的根）
这本书最后一章大概是什么感觉——不是"主角登顶"、"大结局"这种套话，而是**最后一个镜头**大致长什么样。主角最后在哪、做什么、身边有谁、心里想什么。这是给全书所有后面的规划一个远方靶子。

**本段末尾必须明确写出全书 Objective 一句话**：这本书讲完时，主角必须达成一个**可验证的终局状态**（例："从一个杂役修士成为宗门长老并公开父辈冤案的真相"、"从黑户打工妹成为掌控三家皮草公司的老板娘并亲手送前夫进监狱"）。不要写"变强"、"复仇"这类抽象词，要写**一个能被外部观察者判定"达成 / 未达成"的具体状态**。这个 Objective 是全书递归大纲的根——下面 volume_map 的每一卷会分解出这个 O 对应的 Key Results。

=== SECTION: volume_map ===

这是分卷散文地图，**5 段主体 + 1 段节奏原则尾段**。**关键要求：只写到卷级 prose**——写清楚每卷的主题、情绪曲线、卷间钩子、角色阶段目标、卷尾不可逆事件。**禁止指定具体章号任务**（不要写"第 17 章让他回家"这种章级布局）。章级规划是 Phase 3 planner 的职责，架构师只搭骨架、不编章目。

### 段 1：各卷主题与情绪曲线
有几卷？每卷的主题一句话，每卷的情绪曲线一段（哪里压、哪里爽、哪里冷、哪里暖）。不要机械的"第一卷打小怪第二卷打大怪"，写情绪的流动。

### 段 2：卷间钩子与回收承诺（前台/后台双层都要覆盖）
第 1 卷埋什么钩子、在哪一卷回收；第 2 卷埋什么、在哪一卷回收。散文写，不要表格。**只写卷级**（如"第 1 卷埋的身世之谜在第 3 卷回收"），不要写具体章号。

**钩子必须覆盖前台 + 后台两层**（对应 story_frame.段 2 建立的双层故事）：
- 前台钩子：当前卷内 arc 层面的短期钩子（查案谜题、对手身份、资源争夺等），预期在 1-2 卷内回收
- 后台钩子：贯穿全书的主线钩子（幕后真相、身世、体制秘密等），预期在终卷前后回收，核心的 3-7 条属于 core_hook=true

**如果本段只写前台钩子、没有后台钩子暗桩，说明你漏了整本书的引力轴，必须补上。**

### 段 3：各卷 OKR（Objective + Key Results）
用 OKR 递归大纲法分解全书 Objective（story_frame.段 4 末尾定的根 O）：每一卷都必须明确给出：
- **Objective（卷级目标）**：本卷结束时主角必须达成的**可验证状态**，一句话，与全书 Objective 逻辑递进相连（例：全书 O = "成为宗门长老并公开冤案"；卷 1 O = "从杂役转入正式弟子籍并拿到第一份能指向真相的线索"）
- **Key Results（3 条，可量化/可观察）**：支撑该 O 达成的三个关键子成果，每条必须是外部观察者能判定是否完成的状态变更（例 KR1 = "拿下药园执事位置"、KR2 = "与灵安峰结成稳定盟约"、KR3 = "发现父辈案卷的第一半页残片"）。不要写"变强"、"成长"这类模糊 KR

次要角色的阶段性变化也要点到（师父在第 2 卷会死、对手在第 3 卷会黑化等），写在 KR 条目下作为附注。写阶段性，不写完整弧线（完整弧线在 roles）。**每一卷 3 个 KR 是下游 planner 分解章节任务的直接依据——planner 拿到一卷的 3 个 KR 后，按每 3-5 章推进一个 KR 的节奏排章。**

### 段 4：卷尾必须发生的改变
每一卷最后一章必须发生什么不可逆的事——权力结构改变、关系破裂、秘密暴露、主角身份重定位。写散文，一卷一段。**只写"必须发生什么"，不指定是第几章**。

### 段 5：节奏原则（具体化 + 通用）
**这是节奏原则的唯一归宿，不再有独立 rhythm_principles section。** 本段输出 6 条节奏原则。**至少 3 条必须具体化到本书**（例："前 30 章每 5 章一个小爽点"），其余可保留通用原则（例："拒绝机械降神"、"高潮前 3-5 章埋伏笔"）。具体化 + 通用混合是合法的。反面例子："节奏要张弛有度"（废话）。正面例子："前 30 章每 5 章一个小爽点，且小爽点必须落在章末 300 字内"。6 条各写 2-3 句，覆盖（顺序不强制、可替换同权重议题）：
1. 高潮间距——本书大高潮之间最长多少章？（具体化优先）
2. 喘息频率——高压段多长必须插一章喘息？喘息章承担什么任务？
3. 钩子密度——每章章末留钩数量，主钩最多允许悬多少章？
4. 信息释放节奏——主线信息在前 1/3、中段、后 1/3 分别释放多少比例？（可通用）
5. 爽点节奏——爽点间距多少章一个？什么类型为主？（具体化优先）
6. 情感节点递进——情感关系每多少章必须有一次实质推进？

如果外部指令给了内容比例（例如权谋线/感情线各半、事业线/恋爱线的权重），必须在本段写成全书节奏承诺：哪些卷偏哪条线、每个 3-5 章小周期里哪条线必须可见、高潮后哪条线要承担后效。不要只写"保持平衡"。

=== SECTION: roles ===

一人一卡 prose。**主角卡是本书角色弧线的唯一权威来源**——story_frame 不再写主角弧线，writer/planner 都从这里读。用以下格式分隔：

---ROLE---
tier: major
name: <角色名>
---CONTENT---
（这里写散文角色卡，下面的小标题必须全部出现，每段至少 3 行正经散文，不要写表格）

## 核心标签
（3-5 个关键词 + 一句话为什么是这些词）

## 反差细节
（1-2 个与核心标签反差的具体细节——"冷酷杀手但会给流浪猫留鱼骨"。反差细节是人物立体化的公式，必须有。）

## 人物小传（过往经历）
（一段散文，说这个人怎么变成现在这样。童年/重大事件/塑造性格的那件事。只写关键过往，简版。）

## 主角弧线（起点 → 终点 → 代价）
**只有主角必须写本段；其他 major 角色如果弧线分量重也可以写，否则略过。**主角从哪里出发（身份、处境、核心缺陷、一开始最想要什么），到哪里落脚（最终变成什么样的人、拿到/失去什么），为了这个落脚他付出了什么不可逆的代价（关系、身体、信念、某段过去）。不要只写"变强"这种平面变化，要写**内在的位移**。本段是之前 story_frame.段 2 迁移过来的权威位置，写足写实。

## 当前现状（第 0 章初始状态）
（第 0 章时他在哪、做什么、处境如何、最近最烦心的事。**只写角色个人处境**——初始钩子写在 pending_hooks 的 startChapter=0 行；环境/时代锚（如果是需要年份的题材）织进 story_frame.世界观底色。不再有独立的 current_state section。）

## 关系网络
（与主角、与其他重要角色的关系——一句话一条，关系不是标签是动态。）

## 内在驱动
（他想要什么、为什么想要、愿意付出什么代价。）

## 成长弧光
（他在这本书里会经历什么内在位移——变好变坏变复杂，落在哪里。非主角可短可长。）

---ROLE---
tier: major
name: <下一个主要角色>
---CONTENT---
...

（主要角色至少 3 个：主角 + 主要对手 + 主要协作者。建议 2-3 主 + 2-3 辅，不要灌水。质量 > 数量。）

---ROLE---
tier: minor
name: <次要角色名>
---CONTENT---
（次要角色简化版，只需要 4 个小标题：核心标签 / 反差细节 / 当前现状 / 与主角关系，每段 1-2 行即可）

（次要角色 3-5 个，按出场密度给。）

=== SECTION: book_rules ===

输出普通 Markdown，不要 YAML frontmatter，不要 JSON，不要代码块。这里只写给运行时和写手都能读懂的规则卡，不写长篇散文；叙事视角、核心冲突驱动等长说明已经在 story_frame.世界观底色里写过，这里只保留可执行规则。

## 主角
- 名字：<主角名>
- 性格锁：<3-5 个性格关键词，用顿号分隔>
- 行为约束：<3-5 条主角不能违背的行为边界，用顿号分隔>

## 题材锁
- 主类型：${book.genre}
- 禁止混入：<2-3 种禁止混入的文风/体系>

## 叙事人称
<只有当用户明确指定第一人称或第三人称时才写；没指定就写"无">

${gp.numericalSystem ? `## 数值/资源规则
- 核心资源：<核心资源类型>
- 硬上限：<根据设定确定，不能随剧情突破>` : ""}

${gp.eraResearch ? `## 年代限制
- <2-3 条必须符合年代/政策/物价/社会环境的约束>` : ""}

## 禁止事项
- <3-5 条本书禁忌>

=== SECTION: pending_hooks ===

初始伏笔池（Markdown表格），Phase 7 扩展列：
| hook_id | 起始章节 | 类型 | 状态 | 最近推进 | 预期回收 | 回收节奏 | 上游依赖 | 回收卷 | 核心 | 半衰期 | 备注 |

伏笔表规则：
- 第5列必须是纯数字章节号，不能写自然语言描述
- 建书阶段所有伏笔都还没正式推进，所以第5列统一填 0
- 普通种子行不要写 open；尚未被正文真正推进的普通伏笔状态写「暂缓」。只有主线承重、依赖链或跨卷结构需要系统预升级时，运行时才会把它视为活跃伏笔
- 第7列必须填写：立即 / 近期 / 中程 / 慢烧 / 终局 之一
- 第8列「上游依赖」：列出必须在本伏笔之前种下/回收的上游 hook_id，格式如 [H003, H007]；若无依赖填「无」
- 第9列「回收卷」：用自然语言写该伏笔计划在哪一卷哪一段回收（例："第2卷中段"、"终卷终章前"）。不强制解析为章号
- 第10列「核心」：是否主线承重伏笔 true / false。主线承重伏笔一本书最多 3-7 条（主谜团、身世、核心承诺），其余次要伏笔填 false
- 第11列「半衰期」：可选，整数章数。若不填自动按回收节奏推导（立即/近期 = 10、中程 = 30、慢烧/终局 = 80）
- 初始线索放备注列，不放第5列
- **初始世界状态 / 初始敌我关系** 如果有关键信息（例如"主角身上带着父亲的笔记本"、"体制已经开始监视码头"），可以作为 startChapter=0 的种子行录入，备注列说明其"初始状态"属性。

## 最后强调
- 符合${book.platform}平台口味、${gp.name}题材特征
- 主角人设鲜明、行为边界清晰
- 伏笔前后呼应、配角有独立动机不是工具人
- **story_frame / volume_map / roles 必须是散文密度，不要退化成 bullet**
- **book_rules 用普通 Markdown 规则卡，不要 YAML/JSON/代码块，也不要写成长篇散文**
- **不要输出 rhythm_principles 或 current_state 独立 section**——节奏原则合并进 volume_map 尾段；角色初始状态写在 roles.当前现状，初始钩子写在 pending_hooks（startChapter=0 行），环境/时代锚（仅历史/年代/都市重生等需要年份的题材）织进 story_frame.世界观底色，不要硬凑
- **pending_hooks 表必须包含 Phase 7 扩展列——depends_on 标出因果链、pays_off_in_arc 锁定回收大致位置、core_hook 标记主线承重伏笔（3-7 条）、half_life 仅给重点伏笔设置**

## 硬性完结检查（生成前读一遍）
必须依次输出全部 **5 个 SECTION 块**：story_frame → volume_map → roles → book_rules → pending_hooks，不允许因为 story_frame 或 volume_map 写长了就不写后 3 段。哪怕 roles 只列 3 个角色、book_rules 只有 Markdown 小块、pending_hooks 只有 3 行，也要完整输出。只有写完 pending_hooks 最后一行才算交付。`;
}

/**
 * 英文版基础设定 prompt（5 个 SECTION 散文骨架）。
 */
export function buildEnglishFoundationPrompt(
  book: BookConfig,
  gp: GenreProfile,
  genreBody: string,
  contextBlock: string,
  reviewFeedbackBlock: string,
  numericalBlock: string,
  powerBlock: string,
  eraBlock: string,
): string {
  return `You are the architect of this book. Your only job is to produce **prose-density foundation design** — not tables, not schema, not bullet lists. The book's aura comes from your prose density: Phase 3 planner reads sparse memos out of your volume_map only if it was written to chapter-level prose; the writer only produces living characters because your role sheets carry contrast details; the reviewer only catches hard errors because your story_frame set the tonal anchors.${contextBlock}${reviewFeedbackBlock}

## Book metadata
- Platform: ${book.platform}
- Genre: ${gp.name} (${book.genre})
- Target chapters: ${book.targetChapters}
- Chapter length: ${book.chapterWordCount}
- Title: ${book.title}

## Genre body
${genreBody}

## Output constraints
${numericalBlock}
${powerBlock}
${eraBlock}

## Output contract (5 === SECTION: === blocks)

## Deduplication rule (MANDATORY)
Do not duplicate the same fact across sections. The protagonist's arc lives only in roles; world hard-rules live only in story_frame; rhythm principles live only in the last paragraph of volume_map; character initial status lives only in roles.Current_State; initial hooks live only in pending_hooks (start_chapter=0 rows). **When the book is period fiction / historical fanfic / urban reincarnation** — anything pinned to a real year, season, or historic marker — weave the environment/era anchor into story_frame's world-tonal-ground paragraph (e.g. "July 1985, just after the SARS wave"). **For cultivation / high-fantasy / system genres that have no real-world year, skip it entirely** — do not fabricate an era anchor. If a section repeats content that belongs elsewhere, delete it.

## Output budget (over-budget means cut)
- story_frame ≤ 3000 chars
- volume_map ≤ 5000 chars
- roles ≤ 8000 chars total
- book_rules ≤ 1000 chars (ordinary Markdown rules card)
- pending_hooks ≤ 2000 chars

=== SECTION: story_frame ===

Four prose sections, ~600-900 chars each. No tables. No bullet lists. Real paragraphs. **Do NOT write the protagonist's full arc here** — that is owned by roles/主要角色/<protagonist>.md. Use a single-line pointer inside this block (e.g. "The protagonist is X; full arc lives in roles/主要角色/X.md").

## 01_Theme_and_Tonal_Ground
What is this book actually about — not "hero grows from weak to strong" (empty), but a concrete proposition. Then the tonal ground: warm / cold / fierce / severe — which, and why this and not another. End with a one-line pointer to the protagonist role file.

## 02_Core_Conflict_and_Foreground_Background_Story_Layers
The book's main tension — not "good vs evil" but "because A believes X and B believes Y, they will inevitably collide on Z". At least two opponents: one visible, one structural/systemic. Opponents have their own logic.

**This section must explicitly write out the foreground story / background story layers**:
- **Foreground story**: the surface conflict the reader sees every chapter (cases, combat, leveling up, romance, business moves). Each volume / arc has its own visible goal and closure point.
- **Background story**: the hidden machine running through the whole book — the puppet master, conspiracy, origin secret, systemic oppression, fated curse. The reader assembles it from fragments; full payoff lands near the finale.

The two layers must be causally linked, not parallel universes — every foreground conflict should trace back to some gear of the background machine turning. **Foreground-only story collapses into a set of disconnected episodes with no forward pull; background-only story is suffocating and never delivers. Write both in prose here, and name how they interlock.**

## 03_World_Tonal_Ground (hard rules + sensory tone + book-specific rules)
The world's operating rules. 3-5 unbreakable laws written as prose, not bullets. Sensory texture: wet or dry, fast or slow, noisy or quiet — give the writer an anchor. **This paragraph also absorbs the narrative prose that used to live in book_rules (narrative perspective, core conflict driver, book-specific rules).** Write them all here once. Do not repeat them in book_rules.

## 04_Endgame_Direction_and_Book_Objective
What the last chapter roughly feels like. The final shot: where, doing what, around whom, thinking what. A distant target for every planner call downstream.

**End this paragraph with a one-sentence Book Objective** (the root of the recursive OKR outline): when this book is done, the protagonist must reach a **verifiable end-state** (e.g., "rise from errand disciple to sect elder and publicly vindicate the parental case", "go from undocumented migrant worker to running three fur-trade companies and personally putting the ex-husband in prison"). Do NOT use vague words like "grow stronger" or "take revenge" — write a concrete state an outside observer can check "achieved / not achieved". This Book Objective is the root of the full-book OKR outline; volume_map will decompose it per volume below.

=== SECTION: volume_map ===

Prose volume map, **5 sections + 1 closing rhythm paragraph**. **Critical requirement: stay at volume-level prose only** — specify each volume's theme, emotional curve, cross-volume hooks, character stage goals, and volume-end irreversible changes. **Do NOT prescribe chapter-level tasks** (no "chapter 17 sends him home"). Chapter planning is the Phase 3 planner's job; the architect builds the skeleton, not the chapter list.

## 01_Volume_Themes_and_Emotional_Curves
How many volumes? Each volume's theme in one sentence; each volume's emotional curve as a paragraph (where pressured, where rewarding, where cold, where warm). Not mechanical rotation.

## 02_Cross_Volume_Hooks_and_Payoff_Promises (cover BOTH foreground and background layers)
Volume 1 plants hook A, paid off in volume N; volume 2 plants hook B, paid off in volume M. Prose, not tables. **Stay at volume-level** (e.g., "the origin mystery planted in volume 1 pays off in volume 3"); do not specify chapter numbers.

**Hooks must cover BOTH foreground and background layers** (matching the two-layer story established in story_frame.02):
- Foreground hooks: short-range arc-level hooks (case mystery, opponent identity, resource grab), paid off within 1-2 volumes
- Background hooks: full-book main-line hooks (ultimate truth, origin, systemic secret), paid off near the finale. The 3-7 load-bearing ones are core_hook=true

**If this paragraph only carries foreground hooks with no background seeds, you have lost the book's forward pull axis. Add them.**

## 03_Per_Volume_OKRs (Objective + 3 Key Results)
Recursive OKR outline that decomposes the Book Objective (root O set at the end of story_frame.04): every volume must explicitly state:
- **Objective (volume-level goal)**: a **verifiable state** the protagonist must reach by volume end, one sentence, logically chained to the Book Objective (e.g., if Book O = "become sect elder and vindicate the parental case", then Vol 1 O = "move from errand disciple into the registered disciple roster and recover the first lead pointing to the truth")
- **Key Results (3 items, quantifiable / observable)**: three concrete sub-achievements whose completion can be checked by an outside observer (e.g., KR1 = "take over the pharmacy garden steward seat", KR2 = "lock in a stable alliance with Lingan Peak", KR3 = "uncover the first half-page fragment of the parental case file"). No vague KRs like "gets stronger" / "matures".

Supporting characters' stage changes (master dies end of vol 2, opponent breaks bad in vol 3) go as notes under the relevant KR. Stage only — full arc lives in roles. **The 3 KRs per volume are the direct input for the planner: once it sees 3 KRs for a volume, it paces chapter tasks at roughly one KR advanced every 3-5 chapters.**

## 04_Volume_End_Mandatory_Changes
Each volume's last chapter must contain an irreversible event. Prose, one paragraph per volume. **Write what must happen, not which chapter**.

## 05_Rhythm_Principles (concrete + universal)
**This is the single home for rhythm principles — no separate rhythm_principles section exists.** Output 6 rhythm principles. **At least 3 must be concretized for this book** (e.g., "every 5 chapters in the first 30, hit one small payoff"); the rest may stay as universal rules (e.g., "no deus ex machina", "plant the foreshadow 3-5 chapters before the climax"). A mix of concrete + universal is valid. Bad: "rhythm must balance tension and release". Good: "every 5 chapters in the first 30 carries a small payoff landing in the last 300 chars of the chapter". Cover (order flexible, substitutions of equal weight are allowed): (1) climax spacing, (2) breath frequency, (3) hook density, (4) information release pacing, (5) payoff rhythm, (6) relationship advancement — each 2-3 sentences.

If the external instructions specify content proportions (for example politics/romance 50/50 or career/relationship weighting), this paragraph must turn that into a full-book rhythm promise: which volumes lean toward which line, which line must be visible in every 3-5 chapter mini-cycle, and which line carries fallout after climaxes. Do not merely say "keep it balanced."

=== SECTION: roles ===

One-file-per-character prose. **The protagonist card is the single source of truth for the protagonist's arc** — story_frame no longer carries it, and writer/planner both read it here.

---ROLE---
tier: major
name: <character name>
---CONTENT---
## Core_Tags
(3-5 tags + one sentence on why those tags)

## Contrast_Detail
(1-2 concrete details that contradict the core tags — "ice-cold killer but leaves fish bones for stray cats". Contrast detail is the formula for character dimensionality.)

## Back_Story
(Prose paragraph — how this person became who they are. Key past only, keep it lean.)

## Protagonist_Arc (start → end → cost)
**Mandatory for the protagonist; optional for other majors with substantial arcs.** Where they start (identity, situation, core flaw, initial desire); where they land (who they become, what they gain or lose); the irreversible cost they pay for that landing. Show internal displacement, not just growth. This section absorbs what used to live in story_frame.02_Protagonist_Arc.

## Current_State (initial state at chapter 0)
(Where they are at chapter 0, what's on their mind, most recent worry. **Character-only**: initial hooks go in pending_hooks start_chapter=0 rows; environment / era anchors (when the genre has a real year) are woven into story_frame's world-tonal-ground paragraph. No separate current_state section is produced.)

## Relationship_Network
(With protagonist, with other major characters. One line each. Relationships are dynamic, not labels.)

## Inner_Driver
(What they want, why, what they're willing to pay.)

## Growth_Arc
(Internal displacement across the book. Can be short for non-protagonists.)

---ROLE---
tier: major
name: <next major>
---CONTENT---
...

(Aim for 2-3 majors + 2-3 supporting majors. Quality over quantity — do not pad.)

---ROLE---
tier: minor
name: <minor name>
---CONTENT---
(Simplified: only 4 sections — Core_Tags / Contrast_Detail / Current_State / Relationship_to_Protagonist, 1-2 lines each.)

(3-5 minors.)

=== SECTION: book_rules ===

Output ordinary Markdown. Do NOT output YAML frontmatter, JSON, or code fences. This is a compact rules card readable by both runtime and writers; long narrative guidance already lives in story_frame.03_World_Tonal_Ground.

## Protagonist
- Name: <protagonist name>
- Personality lock: <3-5 personality keywords, comma-separated>
- Behavioral constraints: <3-5 behavioral boundaries>

## Genre Lock
- Primary: ${book.genre}
- Forbidden: <2-3 forbidden style/system intrusions>

## Narrative Person
<Write first person or third person ONLY if the user explicitly requested it; otherwise write "none".>

${gp.numericalSystem ? `## Numerical / Resource Rules
- Core resources: <core resource types>
- Hard cap: <setting-specific cap that cannot be broken by plot convenience>` : ""}

${gp.eraResearch ? `## Era Constraints
- <2-3 constraints tied to policy, prices, technology, or social environment>` : ""}

## Prohibitions
- <3-5 book-specific prohibitions>

=== SECTION: pending_hooks ===

Initial hook pool (Markdown table), Phase 7 extended columns:
| hook_id | start_chapter | type | status | last_advanced_chapter | expected_payoff | payoff_timing | depends_on | pays_off_in_arc | core_hook | half_life | notes |

Rules:
- Column 5 is a pure chapter number, not narrative description
- At book creation all planned hooks have last_advanced_chapter = 0
- Ordinary seed rows must not use status "open"; use "deferred" until prose actually advances them. Only load-bearing core / dependency / cross-volume hooks may be pre-promoted by the runtime into active hook debt
- Column 7 must be: immediate / near-term / mid-arc / slow-burn / endgame
- Column 8 (depends_on): upstream hook ids that must be planted / paid off before this one fires, formatted [H003, H007]; write "none" if no upstream
- Column 9 (pays_off_in_arc): free-form prose on where this hook is scheduled to pay off (e.g. "mid of volume 2", "right before the finale"). NOT parsed into chapter numbers
- Column 10 (core_hook): true / false. Core hooks are main-line load-bearing (central mystery, identity, key promise). A book typically has 3-7 cores; everything else is false
- Column 11 (half_life): optional integer chapters. If blank, derived from payoff_timing (immediate/near-term = 10, mid-arc = 30, slow-burn/endgame = 80)
- Put initial signal text in notes, not column 5
- **Initial world / alliance state**: any load-bearing initial condition ("protagonist carries the father's notebook", "the regime already watches the harbor") can be seeded as a start_chapter=0 row with a note-column tag indicating its initial-state nature.

## Final emphasis
- Fit ${book.platform} platform taste and ${gp.name} genre traits
- Protagonist persona clear with sharp behavioral boundaries
- Hooks planted with payoff promises; supporting characters have independent motivation
- **story_frame / volume_map / roles must be prose density — no bullet-list degradation**
- **book_rules is an ordinary Markdown rules card — no YAML, JSON, code fence, or long prose**
- **Do NOT emit rhythm_principles or current_state as separate sections** — rhythm principles live in the last paragraph of volume_map; character initial status goes in roles.Current_State; initial hooks go in pending_hooks (start_chapter=0 rows); environment / era anchors (only when the genre has a real year) are woven into story_frame's world-tonal-ground paragraph
- **pending_hooks table MUST carry Phase 7 extended columns — depends_on spells out the causal chain, pays_off_in_arc locks the approximate payoff location, core_hook marks main-line load-bearing hooks (3-7 per book), half_life only on priority hooks**

## Hard completeness check (read before generating)
You MUST emit all **5 SECTION blocks in order**: story_frame → volume_map → roles → book_rules → pending_hooks. Do NOT stop after story_frame or volume_map just because they ran long. Even if roles lists only 3 characters, book_rules is a small Markdown block, and pending_hooks has only 3 rows, all five must appear. The output is only considered delivered after the last row of pending_hooks is written.`;
}
