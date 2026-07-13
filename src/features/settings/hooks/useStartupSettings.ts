import { useEffect } from "react";
import { getCurrentWindow, PhysicalSize, PhysicalPosition } from "@tauri-apps/api/window";
import { loadSettings, setWindowBounds } from "@/services/settings";

const SAVE_DEBOUNCE_MS = 300;

export function useStartupSettings(): void {
  useEffect(() => {
    let unlisteners: Array<() => void> = [];
    let saveTimer: ReturnType<typeof setTimeout> | null = null;
    const win = getCurrentWindow();

    async function apply() {
      const s = await loadSettings();
      if (s.ui.restoreWindowState && s.window.bounds) {
        const b = s.window.bounds;
        try {
          await win.setSize(new PhysicalSize(b.width, b.height));
          await win.setPosition(new PhysicalPosition(b.x, b.y));
        } catch (e) {
          console.error("[startup] restore window bounds failed", e);
        }
      }
      if (s.ui.restoreWindowState) {
        const saveBounds = () => {
          if (saveTimer) clearTimeout(saveTimer);
          saveTimer = setTimeout(async () => {
            try {
              const [size, pos] = await Promise.all([win.innerSize(), win.outerPosition()]);
              await setWindowBounds({
                width: size.width,
                height: size.height,
                x: pos.x,
                y: pos.y,
              });
            } catch (e) {
              console.error("[startup] save window bounds failed", e);
            }
          }, SAVE_DEBOUNCE_MS);
        };
        const u1 = await win.onResized(() => saveBounds());
        const u2 = await win.onMoved(() => saveBounds());
        unlisteners.push(u1, u2);
      }
    }

    void apply();

    return () => {
      if (saveTimer) clearTimeout(saveTimer);
      unlisteners.forEach((u) => u());
      unlisteners = [];
    };
  }, []);
}