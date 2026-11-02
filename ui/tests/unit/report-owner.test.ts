import { expect, test } from "bun:test";
import { ReportOwner } from "../../src/features/workspace/report-owner";
import type { ReportPort, ReportView } from "../../src/api/reports";

function view(attempt = "first", name = "document", phase: ReportView["phase"] = "ready"): ReportView {
  return { id: (attempt === "first" ? "a" : "b").repeat(32), attempt, name, kind: "document", phase,
    exit_code: null, error: null, idle_expired: false, diagnostics: "", diagnostics_truncated: false };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}
function fixture() {
  const calls: string[] = [];
  let current: ReportView | null = null;
  const port: ReportPort = {
    current: async () => current,
    open: async (attempt, name) => { calls.push(`open:${attempt}`); return current = view(attempt, name); },
    read: async () => current!,
    close: async (id) => { calls.push(`close:${id}`); return current = { ...current!, phase: "exited" }; },
    force: async (id) => { calls.push(`force:${id}`); return current = { ...current!, phase: "exited" }; },
    activity: async () => { calls.push("activity"); return current!; },
    frame: async () => `/views/${"c".repeat(64)}`,
  };
  return { port, calls, set: (value: ReportView | null) => { current = value; } };
}

test("dismiss and reopen serialize around a late open without closing the new inspection", async () => {
  const { port, calls, set } = fixture();
  const started = deferred<void>(), late = deferred<ReportView>();
  const original = port.open;
  port.open = async (attempt, name) => {
    if (attempt !== "first") return original(attempt, name);
    calls.push("open:first"); started.resolve();
    const result = await late.promise; set(result); return result;
  };
  const owner = new ReportOwner(port);
  const first = owner.open("first", "document");
  await started.promise;
  const dismissed = owner.dismiss();
  const second = owner.open("second", "document");
  late.resolve(view());
  await Promise.all([first, dismissed, second]);
  expect(calls).toEqual(["open:first", `close:${"a".repeat(32)}`, "open:second"]);
  expect(owner.snapshot().view?.attempt).toBe("second");
  await owner.refresh();
  expect(owner.snapshot().url).toBe(`/views/${"c".repeat(64)}`);
});

test("a stale readiness response cannot replace a newer view or issue its frame", async () => {
  const { port } = fixture(), owner = new ReportOwner(port);
  await owner.open("first", "document");
  const late = deferred<ReportView>();
  port.read = () => late.promise;
  const refreshing = owner.refresh();
  await owner.open("second", "document");
  late.resolve(view());
  await refreshing;
  expect(owner.snapshot().view?.attempt).toBe("second");
  expect(owner.snapshot().url).toBeNull();
});

test("a lost open acknowledgement stays unresolved even when another view appears", async () => {
  const { port, calls, set } = fixture(), owner = new ReportOwner(port);
  port.open = async () => { throw new Error("connection lost"); };
  await owner.open("first", "document");
  expect(owner.snapshot().uncertain).toBe(true);
  await owner.open("second", "document");
  await owner.recover();
  expect(owner.snapshot().uncertain).toBe(true);
  expect(calls).toEqual([]);
  set(view());
  await owner.recover();
  expect(owner.snapshot().uncertain).toBe(true);
  await owner.dismiss();
  await owner.open("second", "document");
  expect(owner.snapshot().uncertain).toBe(true);
  expect(calls).toEqual([]);
});

test("failed close retains ownership and force requires confirmation plus needs-force state", async () => {
  const { port, calls, set } = fixture(), owner = new ReportOwner(port);
  await owner.open("first", "document");
  port.close = async () => { throw new Error("lost close reply"); };
  await owner.dismiss();
  expect(owner.snapshot().view?.id).toBe("a".repeat(32));
  expect(owner.snapshot().uncertain).toBe(true);
  await owner.force(true);
  expect(calls).toEqual(["open:first"]);
  set(view("first", "document", "needs_force"));
  await owner.recover();
  await owner.force(false);
  expect(calls).toEqual(["open:first"]);
  await owner.force(true);
  expect(calls).toEqual(["open:first", `force:${"a".repeat(32)}`]);
  expect(owner.snapshot().view?.phase).toBe("exited");
});

test("activity is sent only for an inspected ready frame and stops immediately on dismissal", async () => {
  const { port, calls } = fixture(), owner = new ReportOwner(port);
  await owner.open("first", "document");
  await owner.activity();
  expect(calls).toEqual(["open:first"]);
  await owner.refresh();
  await owner.activity();
  const dismissed = owner.dismiss();
  await owner.activity();
  await dismissed;
  expect(calls.filter((call) => call === "activity")).toHaveLength(1);
  expect(owner.snapshot().url).toBeNull();
});


test("lost-close recovery reads the exact retained identity and activity removes a stopped frame", async () => {
  const { port } = fixture(), owner = new ReportOwner(port);
  await owner.open("first", "document");
  await owner.refresh();
  port.activity = async () => view("first", "document", "stopping");
  await owner.activity();
  expect(owner.snapshot().url).toBeNull();
  port.close = async () => { throw new Error("lost close reply"); };
  await owner.dismiss();
  const read: string[] = [];
  port.current = async () => { throw new Error("Must not adopt a different current view"); };
  port.read = async (id) => { read.push(id); return view("first", "document", "exited"); };
  await owner.recover();
  expect(read).toEqual(["a".repeat(32)]);
  expect(owner.snapshot().uncertain).toBe(false);
  expect(owner.snapshot().view?.phase).toBe("exited");
});

test("a definite open refusal allows a later explicit request without an automatic retry", async () => {
  const { ApiError } = await import("../../src/api/client");
  for (const status of [400, 404, 409, 422]) {
    const { port, calls } = fixture(), owner = new ReportOwner(port);
    const open = port.open;
    let requests = 0;
    port.open = async (attempt, name) => {
      if (++requests === 1) throw new ApiError("Report is not available yet.", status);
      return open(attempt, name);
    };
    await owner.open("first", "document");
    expect(owner.snapshot().uncertain).toBe(false);
    expect(owner.snapshot().error).toBe("Report is not available yet.");
    expect(requests).toBe(1);
    await owner.open("first", "document");
    expect(requests).toBe(2);
    expect(calls).toEqual(["open:first"]);
    expect(owner.snapshot().view?.attempt).toBe("first");
  }
});

test("an older refusal cannot overwrite the status of a queued newer selection", async () => {
  const { ApiError } = await import("../../src/api/client");
  const { port } = fixture(), owner = new ReportOwner(port);
  const open = port.open, started = deferred<void>(), release = deferred<void>();
  port.open = async (attempt, name) => {
    if (attempt !== "first") return open(attempt, name);
    started.resolve(); await release.promise;
    throw new ApiError("Old selection unavailable.", 409);
  };
  const first = owner.open("first", "document");
  await started.promise;
  const second = owner.open("second", "document");
  release.resolve();
  await Promise.all([first, second]);
  expect(owner.snapshot().view?.attempt).toBe("second");
  expect(owner.snapshot().error).toBeNull();
});

test("server and authentication failures do not prove an open request was refused", async () => {
  const { ApiError } = await import("../../src/api/client");
  for (const status of [401, 500]) {
    const { port } = fixture(), owner = new ReportOwner(port);
    port.open = async () => { throw new ApiError("No trustworthy acknowledgement.", status); };
    await owner.open("first", "document");
    expect(owner.snapshot().uncertain).toBe(true);
  }
});
