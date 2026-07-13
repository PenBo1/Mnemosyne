import { create } from "zustand";
import type { Route } from "@/app/routes/types";

interface RouteState {
  currentRoute: Route;
  setRoute: (route: Route) => void;
}

const defaultRoute: Route = { name: "chat" };

export const useRouteStore = create<RouteState>((set) => ({
  currentRoute: defaultRoute,
  setRoute: (route) => set({ currentRoute: route }),
}));