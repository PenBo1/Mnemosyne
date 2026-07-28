/**
 * ═══════════════════════════════════════════════════════════════════════════
 * 应用状态上下文 - 全局应用状态管理
 * ═══════════════════════════════════════════════════════════════════════════
 */

import { createContext, useContext, useReducer, type ReactNode } from "react";
import type { AppPage, AppState } from "@/types";
import { DEFAULT_PAGE } from "@/lib/constants";

// ── 类型定义 ────────────────────────────────────────────────────────────────

type Action = { type: "SET_PAGE"; payload: AppPage };

// ── 状态管理 ────────────────────────────────────────────────────────────────

const initialState: AppState = {
  currentPage: DEFAULT_PAGE,
};

function appReducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case "SET_PAGE":
      return { ...state, currentPage: action.payload };
  }
}

// ── Context 定义 ────────────────────────────────────────────────────────────────

const AppStateContext = createContext<AppState>(initialState);
const AppDispatchContext = createContext<React.Dispatch<Action>>(() => {
  throw new Error("AppDispatchContext used without provider");
});

// ── Provider 组件 ────────────────────────────────────────────────────────────────

export function AppProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(appReducer, initialState);

  return (
    <AppStateContext.Provider value={state}>
      <AppDispatchContext.Provider value={dispatch}>
        {children}
      </AppDispatchContext.Provider>
    </AppStateContext.Provider>
  );
}

// ── Hooks ────────────────────────────────────────────────────────────────

export function useAppState() {
  return useContext(AppStateContext);
}

export function useAppDispatch() {
  return useContext(AppDispatchContext);
}