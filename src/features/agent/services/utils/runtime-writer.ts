// Runtime Writer —— 含 I/O 改造。
//
// 运行时治理产物写盘：
//   - writeGovernedRuntimeArtifacts: 写入 3 个文件到 runtimeDir
//     · chapter-NNNN.context.json  (ContextPackage JSON)
//     · chapter-NNNN.rule-stack.json (RuleStack，原为 YAML，迁移后改用 JSON)
//     · chapter-NNNN.trace.json     (ChapterTrace JSON)
//
// 迁移要点：
// 1. import 路径调整：node:fs/promises → @/services/ipc 的 ipcVoid；node:path/join → ../utils/path-utils 的 joinPath。
// 2. I/O 替换：
//    - mkdir(path, { recursive: true }) → await ipcVoid("fs_create_directory", { path })
//    - writeFile(path, content, "utf-8") → await ipcVoid("fs_write_file", { path, content })
// 3. js-yaml 替换：原实现用 `yaml.dump(ruleStack, { lineWidth: 120 })` 写 rule-stack.yaml。
//    RuleStack 是嵌套对象（layers/sections/overrideEdges/activeOverrides 数组+对象），不是简单
//    key:value 形式，自写最小 YAML 序列化会失真。按迁移规则改为 JSON.stringify，文件后缀
//    由 .yaml 改为 .json。返回路径中的 ruleStackPath 也对应改为 .rule-stack.json。
//    （后续如需人类可读的 YAML，可在 Rust 端加 fs_write_yaml 命令。）
// 4. 业务逻辑零改动：保留 Promise.all 并行写盘 + 4 位章节号补零 + 路径返回结构。

import { ipcVoid } from "@/services/ipc";
import { joinPath } from "./path-utils";
import type {
  ChapterTrace,
  ContextPackage,
  RuleStack,
} from "@/features/agent/types/input-governance";

export interface RuntimeArtifactWriteResult {
  readonly contextPath: string;
  readonly ruleStackPath: string;
  readonly tracePath: string;
}

export async function writeGovernedRuntimeArtifacts(params: {
  readonly runtimeDir: string;
  readonly chapterNumber: number;
  readonly contextPackage: ContextPackage;
  readonly ruleStack: RuleStack;
  readonly trace: ChapterTrace;
}): Promise<RuntimeArtifactWriteResult> {
  await ipcVoid("fs_create_directory", { path: params.runtimeDir });

  const chapterSlug = `chapter-${String(params.chapterNumber).padStart(4, "0")}`;
  const contextPath = joinPath(params.runtimeDir, `${chapterSlug}.context.json`);
  // rule-stack 改用 JSON 格式（原为 YAML）
  const ruleStackPath = joinPath(params.runtimeDir, `${chapterSlug}.rule-stack.json`);
  const tracePath = joinPath(params.runtimeDir, `${chapterSlug}.trace.json`);

  await Promise.all([
    ipcVoid("fs_write_file", { path: contextPath, content: JSON.stringify(params.contextPackage, null, 2) }),
    ipcVoid("fs_write_file", { path: ruleStackPath, content: JSON.stringify(params.ruleStack, null, 2) }),
    ipcVoid("fs_write_file", { path: tracePath, content: JSON.stringify(params.trace, null, 2) }),
  ]);

  return {
    contextPath,
    ruleStackPath,
    tracePath,
  };
}
