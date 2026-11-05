/** Where the workspace is, as its address: `/<tool>/<action>/<run>`, each part optional from the
 * right. The tool and action are their stable names from the tool's description (never titles,
 * which are wording); the run is a saved attempt's id. A refresh, Back and a shared link all land
 * on the same place. */
export type Route = { tool?: string; action?: string; run?: string };

export function parseRoute(pathname: string): Route {
  let parts: string[];
  try {
    parts = pathname.split("/").filter(Boolean).map(decodeURIComponent);
  } catch {
    return {};
  }
  const [tool, action, run] = parts;
  return {
    ...(tool ? { tool } : {}),
    ...(tool && action ? { action } : {}),
    ...(tool && action && run ? { run } : {}),
  };
}

export function routePath(route: Route): string {
  const parts: string[] = [];
  if (route.tool) {
    parts.push(route.tool);
    if (route.action) {
      parts.push(route.action);
      if (route.run) parts.push(route.run);
    }
  }
  return "/" + parts.map(encodeURIComponent).join("/");
}
