import { Shutdown } from "./components/Shutdown";
import { Workspace } from "./features/workspace/Workspace";
import { workspaceClient, type Snapshot } from "./api/workspace";
import { Button } from "./components/Button";
import { useEffect, useMemo, useState } from "react";
import { connect } from "./api/session";
import type { LocalApi } from "./api/client";
import { assetClient, type Asset } from "./api/assets";
import { ImageInput } from "./components/ImageInput";
import mark from "./assets/wafflehouse-mark.svg";

export function App() {
  const [snapshot, setSnapshot] = useState<Snapshot>();
  const [connection, setConnection] = useState("Connecting…");
  const [api, setApi] = useState<LocalApi>();
  const [selected, setSelected] = useState<Asset[]>([]);
  const [saved, setSaved] = useState<Asset[]>([]);
  const [libraryError, setLibraryError] = useState<string>();
  const images = useMemo(() => (api ? assetClient(api) : undefined), [api]);
  useEffect(() => {
    let active = true;
    void connect()
      .then((session) => {
        if (active) {
          setConnection(
            session
              ? "Connected locally"
              : "Run wafflehouse in your terminal to open this workspace.",
          );
          setApi(session?.api);
          if (session)
            void workspaceClient(session.api)
              .snapshot()
              .then((value) => {
                if (active) setSnapshot(value);
              })
              .catch((error) => {
                if (active)
                  setLibraryError(
                    error instanceof Error
                      ? error.message
                      : "Unable to load tools.",
                  );
              });
        }
      })
      .catch((error: unknown) => {
        if (active)
          setConnection(
            error instanceof Error
              ? error.message
              : "Unable to connect to the workspace.",
          );
      });
    return () => {
      active = false;
    };
  }, []);
  useEffect(() => {
    let active = true;
    if (images)
      void images
        .list()
        .then((values) => {
          if (active) setSaved(values);
        })
        .catch(() => {
          if (active) setLibraryError("Saved images could not be loaded.");
        });
    return () => {
      active = false;
    };
  }, [images]);
  return (
    <>
      <header className="workspace-header">
        <strong className="workspace-brand"><img src={mark} alt="" width="40" height="24" />wafflehouse</strong>
        <span>Local workspace</span>
        {api && <Shutdown api={api} />}
      </header>
      <main className="workspace-main">
        <h1>Your workshop</h1>
        <p className="workspace-intro">
          Try a tool. Inspect the result. Take it into your pipeline.
        </p>
        <p role="status">{connection}</p>
        {api && !snapshot && <p>Loading workspace…</p>}
        {api && snapshot && snapshot.tools.length > 0 && (
          <Workspace api={api} initial={snapshot} />
        )}
        {images && snapshot && snapshot.tools.length === 0 && (
          <section className="workspace-empty" aria-label="Image library">
            <ImageInput
              label="Reference image"
              min={0}
              max={1}
              value={selected}
              onChange={setSelected}
              preview={images.preview}
              upload={async (file) => {
                const image = await images.upload(file);
                setSaved(await images.list());
                return image;
              }}
            />
            <p>
              Imported images stay in your local library. Removing a selection
              keeps the saved copy.
            </p>
            {libraryError && <p role="alert">{libraryError}</p>}
            {saved.length > 0 && (
              <details>
                <summary>Saved images ({saved.length})</summary>
                <div className="saved-images">
                  {saved.map((image, index) => (
                    <Button key={image.id} onClick={() => setSelected([image])}>
                      Use saved image {index + 1} · {image.width} ×{" "}
                      {image.height}
                    </Button>
                  ))}
                </div>
              </details>
            )}
          </section>
        )}
        {snapshot && snapshot.tools.length === 0 && (
          <section className="workspace-empty" aria-labelledby="setup-heading">
            <h2 id="setup-heading">No tools connected yet</h2>
            <p id="connection-help">
              Connect a tool to try its actions and inspect the results. Tool
              connections are being added in the next milestone.
            </p>
            <div>
              <Button disabled aria-describedby="connection-help">
                Connect a tool
              </Button>
            </div>
          </section>
        )}
      </main>
    </>
  );
}
