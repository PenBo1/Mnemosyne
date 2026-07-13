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