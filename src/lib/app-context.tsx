import { createContext, useContext, useReducer, type ReactNode } from "react";
import type { AppPage, AppState, SettingsTab } from "@/types";
import { DEFAULT_PAGE } from "@/constants";

type Action =
  | { type: "SET_PAGE"; payload: AppPage }
  | { type: "SET_SETTINGS_TAB"; payload: SettingsTab };

const initialState: AppState = {
  currentPage: DEFAULT_PAGE,
  settingsTab: "general",
};

function appReducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case "SET_PAGE":
      return { ...state, currentPage: action.payload };
    case "SET_SETTINGS_TAB":
      return { ...state, settingsTab: action.payload };
  }
}

const AppStateContext = createContext<AppState>(initialState);
const AppDispatchContext = createContext<React.Dispatch<Action>>(() => {
  throw new Error("AppDispatchContext used without provider");
});

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

export function useAppState() {
  return useContext(AppStateContext);
}

export function useAppDispatch() {
  return useContext(AppDispatchContext);
}
