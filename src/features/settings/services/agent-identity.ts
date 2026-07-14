// Agent 身份文件服务 —— 读写 <data_dir>/agents/<role>/{SOUL,CONTEXT,MEMORY}.md。
//
// 后端命令（src-tauri/src/infrastructure/fs/commands.rs）:
// - fs_read_file: 读取文件内容（需 workspace 授权）
// - fs_write_file: 写入文件内容（需 workspace 授权）
// - get_data_dir_path: 返回应用数据目录根路径
//
// 安全说明：lib.rs 启动时已通过 workspace_registry.authorize(data_dir.root())
// 预授权应用数据目录，因此 agents/<role>/ 下的身份文件可被 fs_read_file/fs_write_file 访问。

import { ipc } from "@/services/ipc";
import { getDataDirPath } from "@/features/settings/services/system";

/** 身份文件名常量（对齐后端 IdentityKind::filename） */
export const IDENTITY_FILES = ["SOUL.md", "CONTEXT.md", "MEMORY.md"] as const;
export type IdentityFileName = (typeof IDENTITY_FILES)[number];

/** 拼接身份文件绝对路径 */
function joinPath(dataDir: string, role: string, fileName: string): string {
  const sep = dataDir.endsWith("/") || dataDir.endsWith("\\") ? "" : "/";
  return `${dataDir}${sep}agents/${role}/${fileName}`;
}

/** 读取身份文件内容。文件不存在或读取失败时返回空串（由调用方展示占位符）。 */
export async function loadIdentityFile(
  role: string,
  fileName: IdentityFileName,
): Promise<string> {
  const dataDir = await getDataDirPath();
  const path = joinPath(dataDir, role, fileName);
  try {
    return await ipc<string>("fs_read_file", { path });
  } catch {
    // 文件不存在或未授权时回退到空内容，由 UI 展示占位符
    return "";
  }
}

/** 写入身份文件内容，返回写入字节数 */
export async function saveIdentityFile(
  role: string,
  fileName: IdentityFileName,
  content: string,
): Promise<number> {
  const dataDir = await getDataDirPath();
  const path = joinPath(dataDir, role, fileName);
  return ipc<number>("fs_write_file", { path, content });
}

/** 获取应用数据目录根路径（透传 system 服务，便于组件层直接使用） */
export { getDataDirPath };
