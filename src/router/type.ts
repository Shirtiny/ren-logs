import type { ReactNode } from "react";
import type { RouteObject, LazyRouteFunction } from "react-router";

export interface INavItem {
  path: string;
  label?: string;
  desc?: string;
  icon?: any;
  lazy?: LazyRouteFunction<RouteObject>;
}
