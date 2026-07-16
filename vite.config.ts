import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";

const host = process.env.TAURI_DEV_HOST;

const __dirname = fileURLToPath(new URL(".", import.meta.url));

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],

  // 生产构建 esbuild 优化: 移除 debugger + 噪声 console (保留 error/warn 便于诊断)
  esbuild: {
    drop: ["debugger"],
    pure: [
      "console.log",
      "console.debug",
      "console.info",
      "console.trace",
    ],
  },
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
      "@/features": fileURLToPath(new URL("./src/features", import.meta.url)),
      "@/services": fileURLToPath(new URL("./src/services", import.meta.url)),
      "@/types": fileURLToPath(new URL("./src/types", import.meta.url)),
      "@/locales": fileURLToPath(new URL("./src/locales", import.meta.url)),
      "@/hooks": fileURLToPath(new URL("./src/hooks", import.meta.url)),
      "@/stores": fileURLToPath(new URL("./src/stores", import.meta.url)),
    },
  },

  // 体积优化: treeshake 剥离 console + 精细分包
  build: {
    // Windows 用 chrome120 target, 其他平台 es2022
    target:
      process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome120" : "es2022",
    chunkSizeWarningLimit: 1500,
    rollupOptions: {
      output: {
        manualChunks(id) {
          // Vite preload helper pin 到 react chunk (避免 hoist 到重型 chunk)
          if (id.includes("vite/preload-helper") || id.includes("/vite/dist/"))
            return "react";

          if (!id.includes("node_modules")) return null;

          // 高频样式工具 pin 到 react (eager), 避免被吸入重型 chunk
          if (
            id.includes("/clsx/") ||
            id.includes("/tailwind-merge/") ||
            id.includes("/class-variance-authority/")
          )
            return "react";

          // react 核心 pin 到 react chunk
          if (
            id.includes("/react-dom/") ||
            id.includes("/react/") ||
            id.includes("/scheduler/")
          )
            return "react";

          // @tauri-apps 全家桶独立 chunk (plugin-store/api 较大, 且非首屏必需全部)
          if (id.includes("@tauri-apps/")) return "tauri";

          // radix UI 独立 chunk
          if (id.includes("@radix-ui/") || id.includes("/radix-ui/"))
            return "radix";

          // 重型可视化库独立 chunk (按需懒加载)
          // xyflow + dagre + d3-* 合并到 xyflow chunk: 都是图谱视图的依赖，一起加载更合理
          if (
            id.includes("@xyflow/") ||
            id.includes("/dagre/") ||
            id.includes("@dagrejs/") ||
            id.includes("/d3-")
          )
            return "xyflow";

          // markdown 渲染独立 chunk
          if (
            id.includes("/react-markdown/") ||
            id.includes("/remark-") ||
            id.includes("/rehype-") ||
            id.includes("/unified/") ||
            id.includes("/micromark/") ||
            id.includes("/mdast-") ||
            id.includes("/hast-") ||
            id.includes("/highlight.js/")
          )
            return "markdown";

          // lucide 图标独立 chunk
          if (id.includes("/lucide-react/")) return "icons";

          return null;
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));