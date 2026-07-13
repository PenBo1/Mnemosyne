import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * 将逗号分隔的字符串解析为标签数组。
 *
 * 统一替代散落在各表单组件中的 `input.split(",").map(s => s.trim()).filter(Boolean)` 样板。
 */
export function parseTags(input: string): string[] {
  return input
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}

