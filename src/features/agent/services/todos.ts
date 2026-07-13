// Todo 类型 + 持久化 + 校验。
//
// 持久化用 LazyStore（和 config.json 同机制），存到 mnemosyne-todos.json。
// 按 session 维度隔离：key 为 `todos:<sessionId>`。

import { LazyStore } from "@tauri-apps/plugin-store";

export type TodoStatus = "pending" | "in_progress" | "completed";

export type Todo = {
  id: string;
  title: string;
  description?: string;
  status: TodoStatus;
};

const STORE_PATH = "mnemosyne-todos.json";
const todosKey = (sessionId: string) => `todos:${sessionId}`;

const store = new LazyStore(STORE_PATH, { defaults: {}, autoSave: 200 });

export async function loadTodos(sessionId: string): Promise<Todo[]> {
  return (await store.get<Todo[]>(todosKey(sessionId))) ?? [];
}

export async function saveTodos(
  sessionId: string,
  todos: Todo[],
): Promise<void> {
  await store.set(todosKey(sessionId), todos);
}

export async function deleteTodos(sessionId: string): Promise<void> {
  await store.delete(todosKey(sessionId));
}

export function newTodoId(): string {
  return `t-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
}

/**
 * 校验 todo 列表：
 *  - title 非空
 *  - 最多一个 in_progress（防漂移不变量）
 * 返回 null 表示合法，否则返回错误字符串。
 */
export function validateTodos(todos: Todo[]): string | null {
  let inProgress = 0;
  for (const t of todos) {
    if (!t.title.trim()) return "todo title 不能为空";
    if (t.status === "in_progress") inProgress++;
  }
  if (inProgress > 1) {
    return `最多只能有一个 in_progress todo（当前 ${inProgress} 个）`;
  }
  return null;
}
