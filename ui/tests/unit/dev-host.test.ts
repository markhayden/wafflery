import { expect, test } from "bun:test";
import { forwardedHeaders, returnedHeaders } from "../../dev-host";

const host = new URL("http://127.0.0.1:51448");

test("the dev bridge presents requests as the host's own page", () => {
  const get = forwardedHeaders({ host: "127.0.0.1:5173", authorization: "Bearer x" }, "GET", host);
  expect(get.host).toBe("127.0.0.1:51448");
  expect(get.origin).toBeUndefined();
  expect(get.authorization).toBe("Bearer x");
  const post = forwardedHeaders({ host: "127.0.0.1:5173" }, "POST", host);
  expect(post.origin).toBe("http://127.0.0.1:51448");
  const fetched = forwardedHeaders({ host: "127.0.0.1:5173", origin: "http://127.0.0.1:5173" }, "GET", host);
  expect(fetched.origin).toBe("http://127.0.0.1:51448");
});

test("the dev bridge keeps the host's policies, naming the dev page as their ancestor", () => {
  const policy = "default-src 'none'; frame-src http://127.0.0.1:9; frame-ancestors http://127.0.0.1:51448";
  const out = returnedHeaders({ "content-security-policy": policy, "x-frame-options": "SAMEORIGIN" }, host, "http://127.0.0.1:5173");
  expect(out["content-security-policy"]).toBe("default-src 'none'; frame-src http://127.0.0.1:9; frame-ancestors http://127.0.0.1:5173");
  expect(out["x-frame-options"]).toBe("SAMEORIGIN");
});
