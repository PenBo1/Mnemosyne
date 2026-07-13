import { lazy, Fragment } from "react";
import type { Route } from "./types";
import { pageRegistry } from "./page-registry";
import { layoutRegistry } from "./layout-registry";

export function resolveRoute(route: Route) {
  const config = pageRegistry[route.name];
  if (!config) return null;

  const Layout = layoutRegistry[config.layout] ?? Fragment;
  const Page = lazy(config.loader);

  return { Page, Layout, params: route.params, query: route.query };
}