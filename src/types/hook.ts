// Hook 类型 —— 纯类型定义。
//
// 不含 MemoryDB 类（Mnemosyne 用 Rust rusqlite 替代 node:sqlite）。
// 仅保留 architect/consolidator/runner 共用的数据契约。

export interface StoredHook {
  readonly hookId: string;
  readonly startChapter: number;
  readonly type: string;
  readonly status: string;
  readonly lastAdvancedChapter: number;
  readonly expectedPayoff: string;
  readonly payoffTiming?: string;
  readonly notes: string;
  readonly dependsOn?: ReadonlyArray<string>;
  readonly paysOffInArc?: string;
  readonly coreHook?: boolean;
  readonly halfLifeChapters?: number;
  readonly advancedCount?: number;
  readonly promoted?: boolean;
}

export interface StoredSummary {
  readonly chapter: number;
  readonly title: string;
  readonly characters: string;
  readonly events: string;
  readonly stateChanges: string;
  readonly hookActivity: string;
  readonly mood: string;
  readonly chapterType: string;
}

/**
 * Fact —— 当前状态事实三元组（subject-predicate-object）。
 *
 * 与后端 RetrievedFact DTO（camelCase 序列化）对齐；可选 id 主键服务于
 * MemoryDB 风格的运行时类型，Mnemosyne 实际用 Rust rusqlite 替代。
 */
export interface Fact {
  readonly id?: number;
  readonly subject: string;
  readonly predicate: string;
  readonly object: string;
  readonly validFromChapter: number;
  readonly validUntilChapter: number | null;
  readonly sourceChapter: number;
}