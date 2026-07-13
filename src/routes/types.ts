export interface Route {
  name: string;
  params?: Record<string, string>;
  query?: Record<string, string>;
}

export interface PageConfig {
  loader: () => Promise<{ default: React.ComponentType }>;
  layout: string;
}