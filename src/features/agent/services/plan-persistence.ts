// Plan 队列持久化 service —— 封装 plan-store applyAll 用到的 fs IPC。
//
// 将 store 内直调的 fs_write_file / fs_create_directory 下沉到此 service，
// store 只负责队列状态管理，IPC 调用委托给本模块。
// IPC 约定：使用 @/services/ipc 的 ipc<T>() 包装器（已含 IpcResponse 解包）。

import { ipc } from "@/services/ipc";

/**
 * 写文件到磁盘（plan applyAll 中 write_file / edit / multi_edit 共用）。
 * 返回写入的字节数（fs_write_file 后端返回 number）。
 */
export async function writePlanFile(path: string, content: string): Promise<void> {
  await ipc<number>("fs_write_file", { path, content });
}

/**
 * 创建目录（plan applyAll 中 create_directory 用）。
 * 后端返回 boolean，此处忽略返回值，仅关注成功/失败。
 */
export async function createPlanDirectory(path: string): Promise<void> {
  await ipc<boolean>("fs_create_directory", { path });
}
