import type {ReplayStatus} from "../../api/chain-replays";
import type {BundleUpload} from "../../api/bundle-imports";
import {InvocationBundles} from "./InvocationBundles";
import {CommandHandoff} from "./CommandHandoff";
import styles from "./Pipeline.module.css";
import { useState } from "react";
import type { LocalApi } from "../../api/client";
import type { Attempt } from "../../api/workspace";
import { Button } from "../../components/Button";
import { ActionRow } from "../../components/ActionRow";

/** "Use in your pipeline", from the person's side: run this again yourself, keep a copy, and,
 * rarely, replay several runs as a chain. The run's internals (the recorded call, the resolved
 * settings) are in Run details → Advanced, not here. */
export function Pipeline({
  attempt,
  api,
  onOpenImported,
  onOpenChain,
  activeReplayId,
}: {
  attempt: Attempt;
  api: LocalApi;
  onOpenImported?:((upload:BundleUpload)=>Promise<void>)|undefined;
  onOpenChain?:((status:ReplayStatus)=>Promise<void>)|undefined;
  activeReplayId?:string|undefined;
}) {
  const [images, setImages] = useState(true),
    [busy, setBusy] = useState(false),
    [notice, setNotice] = useState("");
  const assets = [...Object.values(attempt.inputs).flat(), ...attempt.outputs];
  const unique = [...new Map(assets.map((value) => [value.asset.id, value.asset])).values()];
  const megabytes = unique.reduce((sum, asset) => sum + asset.size, 0) / 1024 / 1024;
  const tool = attempt.declaration.document?.tool;
  const maySpend = attempt.declaration.document?.actions.find(action => action.name === attempt.request.action)?.cost.may_spend;
  async function download() {
    setBusy(true);
    setNotice("");
    try {
      const response = await api.request(`/api/attempts/${encodeURIComponent(attempt.id)}/export`, {
        method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ include_images: images }),
      });
      const url = URL.createObjectURL(await response.blob());
      const link = document.createElement("a");
      link.href = url;
      link.download = "wafflery-evidence.tar.gz";
      link.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      setNotice("Archive downloaded.");
    } catch (failure) {
      setNotice(failure instanceof Error ? failure.message : "The archive could not be downloaded.");
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className={styles.pipeline}>
      <section className={styles.section} aria-labelledby="pipeline-terminal">
        <header><h3 id="pipeline-terminal">Run it from your terminal</h3>
          <p>Repeat this exact run outside Wafflery, with the same inputs and settings.</p></header>
        <CommandHandoff id={attempt.id} api={api} tool={tool} maySpend={maySpend}
          revision={JSON.stringify([attempt.execution,!!attempt.result,attempt.preservation.contract])}/>
      </section>

      <section className={styles.section} aria-labelledby="pipeline-copy">
        <header><h3 id="pipeline-copy">Keep a copy</h3>
          <p>One file with this run's settings and results, to keep or share. It doesn't run anything.</p></header>
        <label className={styles.check}>
          <input type="checkbox" checked={images} onChange={(event) => setImages(event.target.checked)} />
          <span>Include image files <small>{unique.length === 0 ? "this run has none" : `${unique.length} ${unique.length === 1 ? "image" : "images"} · ${megabytes < 0.1 ? "under 0.1" : megabytes.toFixed(1)} MB`}</small></span>
        </label>
        {!images && unique.length > 0 && <p className={styles.notice}>References only: the image files are left out, and you'd supply them yourself.</p>}
        <ActionRow>
          <Button busy={busy} disabled={attempt.preservation.contract === "pending"} onClick={() => void download()}>Download archive</Button>
        </ActionRow>
      </section>

      <details className={styles.more}>
        <summary>Replay several runs as a chain</summary>
        <p>Pick saved runs in order. Wafflery bundles them into one file the tool can replay, each step using the previous step's output.</p>
        <InvocationBundles api={api} attempt={attempt} onOpenImported={onOpenImported} onOpenChain={onOpenChain} activeReplayId={activeReplayId}/>
      </details>

      {notice && <p role="status" className={styles.notice}>{notice}</p>}
    </div>
  );
}
