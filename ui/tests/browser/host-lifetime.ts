import type { ChildProcess } from "node:child_process";

export type ShutdownControls = {
  status: (signal: AbortSignal) => Promise<string>;
  force: (signal: AbortSignal) => Promise<void>;
};
type Exit = { code: number | null; signal: NodeJS.Signals | null; error?: Error };

// Capture closure at spawn time, before a browser or teardown can stop the process.
// The exitCode property and disappearing sockets are not process-lifetime handshakes.
export class HostLifetime {
  private readonly closed: Promise<Exit>;
  private didExit = false;
  private didClose = false;
  private stopping: Promise<void> | undefined;

  constructor(private readonly child: ChildProcess) {
    this.closed = new Promise((resolve) => {
      child.once("exit", () => { this.didExit = true; });
      child.once("close", (code, signal) => {
        this.didClose = true;
        resolve({ code, signal });
      });
      child.once("error", (error) => {
        this.didClose = true;
        resolve({ code: null, signal: null, error });
      });
    });
  }
  stop(controls?: ShutdownControls): Promise<void> {
    this.stopping ??= this.stopOwned(controls);
    return this.stopping;
  }
  private async stopOwned(controls?: ShutdownControls): Promise<void> {
    const abort = new AbortController();
    let timer: ReturnType<typeof setTimeout> | undefined;
    const deadline = new Promise<never>((_, reject) => {
      // Shorter than the browser test deadline; never claim a failed teardown succeeded.
      timer = setTimeout(() => {
        abort.abort();
        reject(new Error("Owned host did not close during teardown"));
      }, 10_000);
    });
    const closed = this.closed.then((exit) => {
      if (exit.error) throw exit.error;
      if (exit.code !== 0 || exit.signal !== null) {
        throw new Error(`Owned host exited abnormally (code ${exit.code}, signal ${exit.signal})`);
      }
    });
    const monitor = async () => {
      if (this.didClose) return closed;
      if (!this.didExit) this.child.kill("SIGTERM");
      if (!controls) return closed;
      try {
        while (!this.didClose) {
          const phase = await Promise.race([controls.status(abort.signal), closed]);
          if (phase === undefined || phase === "stopped") return closed;
          if (phase === "waiting") {
            await Promise.race([controls.force(abort.signal), closed]);
            return closed;
          }
          await Promise.race([new Promise((resolve) => setTimeout(resolve, 100)), closed]);
        }
      } catch {
        // Closing removes IPC/HTTP before Node necessarily delivers exit/close. Success is
        // still conditional on a clean close, never on swallowing a connection error.
        return closed;
      }
      return closed;
    };
    try {
      await Promise.race([monitor(), deadline]);
    } finally {
      clearTimeout(timer);
      abort.abort();
    }
  }
}

// Obtain this session while the host is ready, never after asking it to shut down.
export async function shutdownControls(origin: string, bootstrap: string): Promise<ShutdownControls> {
  const response = await fetch(`${origin}/api/session`, {
    method: "POST",
    headers: { Origin: origin, "Content-Type": "application/json" },
    body: JSON.stringify({ bootstrap }),
  });
  if (!response.ok) throw new Error(`Host teardown session failed (${response.status})`);
  const session = await response.json() as { token: string };
  if (typeof session.token !== "string") throw new Error("Host teardown session is missing its credential");
  const headers = { Origin: origin, Authorization: `Bearer ${session.token}`, "Content-Type": "application/json" };
  return {
    async status(signal) {
      const response = await fetch(`${origin}/api/host/shutdown`, { headers, signal });
      if (!response.ok) throw new Error(`Host shutdown status failed (${response.status})`);
      return (await response.json() as { phase: string }).phase;
    },
    async force(signal) {
      const response = await fetch(`${origin}/api/host/force-quit`, { method: "POST", headers, body: "{}", signal });
      if (!response.ok) throw new Error(`Host force quit failed (${response.status})`);
    },
  };
}
