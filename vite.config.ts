import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";
import { visualizer } from "rollup-plugin-visualizer";

const host = process.env.TAURI_DEV_HOST;

const __dirname = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig(() => ({
  plugins: [
    react(),
    tailwindcss(),
    visualizer({
      filename: "dist/stats.html",
      open: false,
      gzipSize: true,
      brotliSize: true,
    }),
  ],

  esbuild: {
    drop: ["debugger"],
    pure: [
      "console.log",
      "console.debug",
      "console.info",
      "console.trace",
      "console.error",
      "console.warn",
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

  build: {
    target:
      process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome120" : "es2022",
    chunkSizeWarningLimit: 1500,
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL("./index.html", import.meta.url)),
        "process-monitor": fileURLToPath(
          new URL("./src/process-monitor/index.html", import.meta.url)
        ),
        "log-viewer": fileURLToPath(
          new URL("./src/log-viewer/index.html", import.meta.url)
        ),
      },
      output: {
        manualChunks(id) {
          if (id.includes("vite/preload-helper") || id.includes("/vite/dist/"))
            return "react";

          if (!id.includes("node_modules")) return null;

          if (
            id.includes("/clsx/") ||
            id.includes("/tailwind-merge/") ||
            id.includes("/class-variance-authority/")
          )
            return "react";

          if (
            id.includes("/react-dom/") ||
            id.includes("/react/") ||
            id.includes("/scheduler/")
          )
            return "react";

          if (id.includes("@tauri-apps/")) return "tauri";

          if (id.includes("@radix-ui/") || id.includes("/radix-ui/"))
            return "radix";

          if (
            id.includes("@xyflow/") ||
            id.includes("/d3-")
          )
            return "xyflow";

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

          if (id.includes("/recharts/")) return "recharts";

          if (id.includes("/zustand/") || id.includes("/use-sync-external-store/"))
            return "vendor-zustand";

          if (id.includes("/lucide-react/")) return "icons";

          return null;
        },
      },
    },
  },

  clearScreen: false,
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
      ignored: ["**/src-tauri/**"],
    },
  },
}));