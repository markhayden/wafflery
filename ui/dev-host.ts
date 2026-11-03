// Development only: lets Vite's dev server (hot reload) front a running `wafflery start` host.
// The host keeps every check it has. This plugin finds the host through its private control
// socket (the record in <library>/host.json, same user only), forwards /api and /views to it as
// if the browser were on the host's own origin, and hands the dev page a fresh one-use bootstrap
// so a restarted host is rejoined without a launcher link. It is never part of a built UI.
import { readFileSync, realpathSync } from "node:fs";
import { request } from "node:http";
import { connect } from "node:net";
import type { IncomingMessage, OutgoingHttpHeaders, ServerResponse } from "node:http";
import type { Plugin } from "vite";

type Reply = { origin?: string; bootstrap?: string };

/** One request on the host's control socket (the protocol `wafflery` itself uses). */
export function control(library: string, operation: "status" | "bootstrap"): Promise<Reply> {
  const root = realpathSync(library);
  const record = JSON.parse(readFileSync(`${root}/host.json`, "utf8")) as { socket?: unknown };
  if (typeof record.socket !== "string") throw new Error("host record names no control socket");
  return new Promise((resolve, reject) => {
    const socket = connect(record.socket as string);
    const chunks: Buffer[] = [];
    socket.setTimeout(2000, () => socket.destroy(new Error("host control timed out")));
    socket.on("error", reject);
    socket.on("data", (chunk: Buffer) => chunks.push(chunk));
    socket.on("end", () => {
      try {
        const reply = JSON.parse(Buffer.concat(chunks).toString("utf8")) as Reply & { product?: string };
        if (reply.product !== "wafflery" || typeof reply.origin !== "string") throw new Error("not a wafflery host");
        resolve(reply);
      } catch (error) { reject(error); }
    });
    socket.end(JSON.stringify({ protocol: 1, product: "wafflery", root, operation }));
  });
}

/** Request headers as the host expects them from its own page: its Host, and its Origin. */
export function forwardedHeaders(headers: IncomingMessage["headers"], method: string, host: URL): OutgoingHttpHeaders {
  const out: OutgoingHttpHeaders = { ...headers, host: host.host };
  if (headers.origin !== undefined || (method !== "GET" && method !== "HEAD")) out.origin = host.origin;
  return out;
}

/** Response headers for the dev page: the host's policies, naming the dev origin where they name the host's. */
export function returnedHeaders(headers: IncomingMessage["headers"], host: URL, page: string): OutgoingHttpHeaders {
  const out: OutgoingHttpHeaders = { ...headers };
  const policy = out["content-security-policy"];
  if (typeof policy === "string") out["content-security-policy"] = policy.replaceAll(host.origin, page);
  return out;
}

function unavailable(response: ServerResponse) {
  if (response.headersSent) { response.destroy(); return; }
  response.writeHead(503, { "content-type": "application/json", "cache-control": "no-store" });
  response.end(JSON.stringify({ error: { code: "host_unavailable", message: "The Wafflery host is starting or restarting." } }));
}

export function waffleryDevHost(library: string): Plugin {
  let origin: string | undefined;
  const target = async () => new URL(origin ??= (await control(library, "status")).origin!);
  return {
    name: "wafflery-dev-host",
    apply: "serve",
    configureServer(server) {
      server.middlewares.use((incoming, response, next) => {
        const url = incoming.url ?? "/";
        if (url === "/__wafflery/bootstrap") {
          control(library, "bootstrap").then((reply) => {
            origin = reply.origin;
            response.writeHead(200, { "content-type": "application/json", "cache-control": "no-store" });
            response.end(JSON.stringify({ bootstrap: reply.bootstrap }));
          }, () => { origin = undefined; unavailable(response); });
          return;
        }
        if (!url.startsWith("/api/") && !url.startsWith("/views/")) { next(); return; }
        const method = incoming.method ?? "GET";
        const page = `http://${incoming.headers.host}`;
        target().then((host) => {
          const upstream = request({
            hostname: host.hostname, port: host.port, method, path: url,
            headers: forwardedHeaders(incoming.headers, method, host),
          }, (reply) => {
            response.writeHead(reply.statusCode ?? 502, returnedHeaders(reply.headers, host, page));
            reply.pipe(response);
          });
          upstream.on("error", () => { origin = undefined; unavailable(response); });
          incoming.pipe(upstream);
        }, () => { origin = undefined; unavailable(response); });
      });
    },
  };
}
