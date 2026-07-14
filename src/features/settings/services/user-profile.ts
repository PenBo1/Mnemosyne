// 用户画像服务 —— 暴露 user_get_profile / user_update_profile IPC 给前端。
//
// 后端命令（src-tauri/src/domain/user/commands.rs）:
// - user_get_profile: 读取当前用户画像
// - user_update_profile: 更新用户画像（整体覆盖）

import { ipc } from "@/services/ipc";
import type { UserProfile } from "@/features/settings/types/user-profile";

/** 读取当前用户画像 */
export async function getUserProfile(): Promise<UserProfile> {
  return ipc<UserProfile>("user_get_profile", {});
}

/** 更新用户画像（整体覆盖，返回更新后的画像） */
export async function updateUserProfile(profile: UserProfile): Promise<UserProfile> {
  return ipc<UserProfile>("user_update_profile", { profile });
}
