import { expect, test } from "bun:test";
import { EventEmitter } from "node:events";
import type { ChildProcess } from "node:child_process";
import { HostLifetime } from "../browser/host-lifetime";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}
function fixture() {
  const emitter = new EventEmitter();
  let signals = 0;
  const child = Object.assign(emitter, {
    exitCode: null as number | null, signalCode: null as NodeJS.Signals | null,
    kill: () => { signals++; return true; },
  });
  const lifetime = new HostLifetime(child as unknown as ChildProcess);
  return {
    lifetime, signals: () => signals,
    exit(code: number | null = 0, signal: NodeJS.Signals | null = null) {
      child.exitCode = code; child.signalCode = signal;
      emitter.emit("exit", code, signal);
    },
    close(code: number | null = 0, signal: NodeJS.Signals | null = null) {
      emitter.emit("close", code, signal);
    },
  };
}
test("shutdown transport loss waits for confirmed process closure, not exitCode timing", async () => {
  const host = fixture();
  const called = deferred<void>();
  let finished = false;
  const stop = host.lifetime.stop({
    status: async () => { called.resolve(); throw new Error("connect ENOENT: socket already unlinked"); },
    force: async () => { throw new Error("must not force a disappearing host"); },
  }).then(() => { finished = true; });
  // Attach the rejection before waiting so the red implementation cannot leak it.
  const outcome = stop.then(() => null, (error: unknown) => error);
  await Promise.race([called.promise, outcome]);
  expect(host.signals()).toBe(1);
  await Promise.resolve();
  expect(finished).toBe(false);
  host.exit();
  await Promise.resolve();
  expect(finished).toBe(false);
  host.close();
  expect(await outcome).toBeNull();
});
test("a host closed by the UI needs no signal or new control request", async () => {
  const host = fixture(); host.exit(); host.close();
  await host.lifetime.stop({
    status: async () => { throw new Error("control is gone"); },
    force: async () => { throw new Error("control is gone"); },
  });
  expect(host.signals()).toBe(0);
});
test("force follows waiting and repeated stop calls share one lifecycle", async () => {
  const host = fixture(); let forces = 0;
  const controls = {
    status: async () => "waiting",
    force: async () => { forces++; host.exit(); host.close(); },
  };
  await Promise.all([host.lifetime.stop(controls), host.lifetime.stop(controls)]);
  expect(forces).toBe(1); expect(host.signals()).toBe(1);
});
test("signal termination is observed even though exitCode stays null, and is reported", async () => {
  const host = fixture(); host.exit(null, "SIGKILL"); host.close(null, "SIGKILL");
  await expect(host.lifetime.stop()).rejects.toThrow("SIGKILL");
  expect(host.signals()).toBe(0);
});
