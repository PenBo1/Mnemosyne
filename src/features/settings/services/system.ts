import { ipc } from "@/services/ipc";

/** 获取应用数据目录根路径 */
export async function getDataDirPath(): Promise<string> {
  return ipc<string>("get_data_dir_path");
}
