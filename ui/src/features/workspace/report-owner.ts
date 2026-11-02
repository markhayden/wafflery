import { ApiError } from "../../api/client";
import type { ReportPort, ReportView } from "../../api/reports";
export type ReportSnapshot = {
  view: ReportView | null; url: string | null; pending: boolean; uncertain: boolean; error: string | null;
};
/** One owner per workspace, retained across drawer mounts and selected saved runs. */
export class ReportOwner {
  private state: ReportSnapshot = {view:null, url:null, pending:false, uncertain:false, error:null};
  private listeners = new Set<() => void>();
  private queue = Promise.resolve();
  private generation = 0;
  private uncertainty: "open" | "close" | null = null;
  private inspecting = false;
  private refreshing = false;
  constructor(private readonly port: ReportPort) {}
  snapshot = () => this.state;
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; };
  private update(change: Partial<ReportSnapshot>) {
    this.state = {...this.state, ...change};
    for (const listener of this.listeners) listener();
  }
  private mutate(operation: (generation: number) => Promise<void>) {
    const generation = ++this.generation;
    this.update({url:null, pending:true, error:null});
    this.queue = this.queue.then(() => operation(generation)).catch((error: unknown) => {
      if (generation === this.generation)
        this.update({error:error instanceof Error ? error.message : "Report request failed."});
    }).finally(() => {
      if (generation === this.generation) this.update({pending:false});
    });
    return this.queue;
  }
  private async release() {
    const view = this.state.view;
    if (!view || view.phase === "exited") return;
    this.uncertainty = "close";
    this.update({uncertain:true}); // Keep the identity if acknowledgement is lost.
    const closed = await this.port.close(view.id);
    this.uncertainty = null;
    this.update({view:closed, url:null, uncertain:false});
    if (closed.phase !== "exited") throw new Error("The previous report is still closing. Check its status before opening another.");
  }
  open(attempt: string, name: string) {
    this.inspecting = true;
    return this.mutate(async (generation) => {
      if (generation !== this.generation) return;
      if (this.state.uncertain) throw new Error("Check the unresolved report before opening another.");
      await this.release();
      if (generation !== this.generation) return;
      const existing = await this.port.current(attempt);
      this.update({view:existing});
      if (existing && existing.phase !== "exited"
        && (existing.name !== name || !["starting", "ready"].includes(existing.phase))) await this.release();
      if (generation !== this.generation) return;
      this.uncertainty = "open";
      this.update({uncertain:true});
      let opened: ReportView;
      try { opened = await this.port.open(attempt, name); }
      catch (error) {
        // These host refusals occur before a renderer is spawned; only explicit retry is allowed.
        if (error instanceof ApiError && [400, 404, 409, 422].includes(error.status)) {
          this.uncertainty = null;
          this.update({uncertain:false});
        }
        throw error;
      }
      this.uncertainty = null;
      this.update({view:opened, uncertain:false});
      // The shared queue prevents a reopened drawer adopting this ID before cleanup.
      if (generation !== this.generation) await this.release();
    });
  }
  inspect() { this.inspecting = true; }
  dismiss() {
    this.inspecting = false;
    return this.close();
  }
  close() {
    return this.mutate(async () => {
      if (this.uncertainty === "open") throw new Error("An open request is unresolved. Quit and reopen Wafflery before opening another report.");
      await this.release();
    });
  }
  recover() {
    return this.mutate(async () => {
      // Even a positive lookup cannot prove a lost open request has settled.
      if (this.uncertainty === "open") throw new Error("An open request is unresolved. Quit and reopen Wafflery before opening another report.");
      if (!this.state.view) return;
      const view = await this.port.read(this.state.view.id);
      this.uncertainty = null;
      this.update({view, uncertain:false});
    });
  }
  force(confirmed: boolean) {
    if (!confirmed || this.state.uncertain || this.state.view?.phase !== "needs_force") return Promise.resolve();
    return this.mutate(async () => {
      const view = this.state.view;
      if (!view || view.phase !== "needs_force" || this.state.uncertain) return;
      this.uncertainty = "close";
      this.update({uncertain:true});
      const closed = await this.port.force(view.id);
      this.uncertainty = null;
      this.update({view:closed, uncertain:false});
    });
  }
  async refresh() {
    const view = this.state.view, generation = this.generation;
    if (!this.inspecting || this.state.pending || this.state.uncertain || !view || this.refreshing) return;
    this.refreshing = true;
    try {
      const observed = await this.port.read(view.id);
      if (generation !== this.generation) return;
      this.update({view:observed, ...(observed.phase !== "ready" ? {url:null} : {})});
      if (observed.phase === "ready" && !this.state.url) {
        const url = await this.port.frame(observed.id);
        if (generation === this.generation) this.update({url, error:null});
      }
    } catch (error) {
      if (generation === this.generation) this.update({url:null, error:error instanceof Error ? error.message : "Report status unavailable."});
    } finally { this.refreshing = false; }
  }
  async activity() {
    const view = this.state.view, generation = this.generation;
    if (!this.inspecting || this.state.pending || this.state.uncertain || !this.state.url || view?.phase !== "ready") return;
    try {
      const observed = await this.port.activity(view.id);
      if (generation === this.generation) this.update({view:observed, ...(observed.phase !== "ready" ? {url:null} : {})});
    } catch { await this.refresh(); }
  }
}
