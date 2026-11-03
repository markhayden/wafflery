import {CommandHandoff} from "./CommandHandoff";
import {savedInvocation} from "./invocation";
import styles from "./Evidence.module.css";
import { useState } from "react";
import type { LocalApi } from "../../api/client";
import type { Attempt } from "../../api/workspace";
import { Button } from "../../components/Button";
export function Pipeline({
  attempt,
  api,
}: {
  attempt: Attempt;
  api: LocalApi;
}) {
  const [view, setView] = useState<"command"|"invocation"|"call" | "setup" | "archive">("command");
  const [images, setImages] = useState(true),
    [busy, setBusy] = useState(false),
    [notice, setNotice] = useState("");
  const assets = [...Object.values(attempt.inputs).flat(), ...attempt.outputs];
  const unique = [
    ...new Map(assets.map((value) => [value.asset.id, value.asset])).values(),
  ];
  const setup = {
    tool: attempt.declaration.document?.tool,
    contract: attempt.declaration.document?.contract,
    mcp_protocol: "2025-11-25",
    parameters: attempt.request.params,
    inputs: attempt.inputs,
    preservation: attempt.preservation,
  };
  const text = JSON.stringify(view === "call" ? attempt.wire : setup, null, 2);
  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      setNotice("Copied saved evidence.");
    } catch {
      setNotice(
        "Clipboard access is unavailable. Select and copy the text below.",
      );
    }
  }
  async function download() {
    setBusy(true);
    setNotice("");
    try {
      const response = await api.request(
        `/api/attempts/${encodeURIComponent(attempt.id)}/export`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ include_images: images }),
        },
      );
      const url = URL.createObjectURL(await response.blob());
      const link = document.createElement("a");
      link.href = url;
      link.download = "wafflery-evidence.tar.gz";
      link.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
      setNotice("Evidence archive downloaded.");
    } catch (failure) {
      setNotice(
        failure instanceof Error
          ? failure.message
          : "Evidence could not be downloaded.",
      );
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className={styles.inspector}>
      <nav aria-label="Pipeline views">
        <Button aria-pressed={view==="command"} onClick={()=>setView("command")}>Command</Button>
        <Button aria-pressed={view==="invocation"} onClick={()=>setView("invocation")}>Resolved invocation</Button>
        <Button aria-pressed={view === "call"} onClick={() => setView("call")}>
          Recorded MCP call
        </Button>
        <Button
          aria-pressed={view === "setup"}
          onClick={() => setView("setup")}
        >
          Inputs & setup
        </Button>
        <Button
          aria-pressed={view === "archive"}
          onClick={() => setView("archive")}
        >
          Evidence archive
        </Button>
      </nav>
      {view==="command"?<CommandHandoff id={attempt.id} api={api} tool={attempt.declaration.document?.tool} maySpend={attempt.declaration.document?.actions.find(action=>action.name===attempt.request.action)?.cost.may_spend} revision={JSON.stringify([attempt.execution,!!attempt.result,attempt.preservation.contract])}/>:view==="invocation"?<section><pre aria-label="Pipeline invocation">{JSON.stringify(savedInvocation(attempt.result),null,2)}</pre></section>:view !== "archive" ? (
        <>
          <p>
            {view === "call"
              ? "Exact recorded request from this run. Paths and request IDs belong to its original host session."
              : "Saved settings and image references from this run."}
          </p>
          <pre aria-label="Pipeline evidence">
            {text ?? "No recorded call is available for this attempt."}
          </pre>
          <Button disabled={!text} onClick={() => void copy()}>
            Copy {view === "call" ? "recorded call" : "setup"}
          </Button>
          <p>
            Use Command for the saved invocation's local CLI handoff. Portable replay bundles
            are separate from this recorded evidence.
          </p>
        </>
      ) : (
        <>
          <p>
            The archive contains a manifest, saved run evidence and optionally
            its input/output images. This is an evidence package, not a portable
            native replay bundle.
          </p>
          <label>
            <input
              type="checkbox"
              checked={images}
              onChange={(event) => setImages(event.target.checked)}
            />{" "}
            Include image files
          </label>
          <p>
            {images
              ? `${unique.length} image files · ${(unique.reduce((sum, asset) => sum + asset.size, 0) / 1024 / 1024).toFixed(2)} MiB before compression`
              : "References only: image bytes are omitted and must be supplied separately."}
          </p>
          <ul>
            {unique.map((asset) => (
              <li key={asset.id}>
                {asset.width} × {asset.height} · {asset.sha256.slice(0, 12)} ·{" "}
                {(asset.size / 1024).toFixed(1)} KB
              </li>
            ))}
          </ul>
          <p>
            Up to 256 MiB of image/evidence data. Provider environment, native
            dependencies and tool reports are not included.
          </p>
          <Button
            busy={busy}
            disabled={attempt.preservation.contract === "pending"}
            onClick={() => void download()}
          >
            Download evidence archive
          </Button>
        </>
      )}
      {notice && <p role="status">{notice}</p>}
    </div>
  );
}
