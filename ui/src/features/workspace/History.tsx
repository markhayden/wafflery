import { useEffect, useState } from "react";
import { workspaceClient, type Attempt, type AttemptSummary, type RegisteredTool, type SavedImage } from "../../api/workspace";
import type { Asset } from "../../api/assets";
import { object } from "../actions/model";
import { Button } from "../../components/Button";
import { ActionRow } from "../../components/ActionRow";
import { ImageTile } from "../../components/ImageThumb";
import { ListRow, RowList } from "../../components/ListRow";
import { purposeParts } from "./activities";
import { RunState } from "./RunStatus";
import styles from "./History.module.css";

const time = (ms: number) => new Date(ms).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
const day = (ms: number) => new Date(ms).toDateString();
function dayHeading(ms: number) {
  const date = new Date(ms), today = new Date(), yesterday = new Date(Date.now() - 86_400_000);
  const full = date.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
  return date.toDateString() === today.toDateString() ? { name: "Today", full }
    : date.toDateString() === yesterday.toDateString() ? { name: "Yesterday", full }
    : { name: date.toLocaleDateString(undefined, { weekday: "long" }), full };
}

/** Saved runs, newest first, grouped by day. Each row is the action's name and what it does (as the
 * operations menu says it), when it ran and how it ended; its chevron shows the images in and out
 * and the settings without opening the run. */
export function History({ client, onOpen, tools = [], preview }: {
  client: ReturnType<typeof workspaceClient>;
  onOpen: (id: string) => void;
  tools?: RegisteredTool[];
  preview?: ((asset: Asset) => Promise<Blob>) | undefined;
}) {
  const [items, setItems] = useState<AttemptSummary[]>([]);
  const [cursor, setCursor] = useState<number | null>(null);
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let active = true;
    setBusy(true);
    void client.history()
      .then((page) => { if (active) { setItems(page.items); setCursor(page.cursor); } })
      .catch((failure) => { if (active) setError(String(failure)); })
      .finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, [client]);
  async function more() {
    if (cursor === null || busy) return;
    setBusy(true);
    setError(undefined);
    try {
      const page = await client.history(cursor);
      setItems((previous) => [...previous, ...page.items]);
      setCursor(page.cursor);
    } catch (failure) {
      setError(String(failure));
    } finally {
      setBusy(false);
    }
  }
  const describe = (item: AttemptSummary) => {
    const declaration = tools.find((tool) => tool.id === item.tool)?.declaration?.document;
    const action = declaration?.actions.find((action) => action.name === item.action);
    return { title: action?.title ?? item.action, about: action ? purposeParts(action.purpose).summary : undefined, tool: declaration?.tool.name ?? item.tool };
  };
  const days = items.reduce<{ key: string; items: AttemptSummary[] }[]>((groups, item) => {
    const key = day(item.created_ms), last = groups[groups.length - 1];
    if (last?.key === key) last.items.push(item); else groups.push({ key, items: [item] });
    return groups;
  }, []);
  return (
    <section aria-label="Saved history" className={styles.history}>
      {error && <p role="alert">{error}</p>}
      {!busy && !items.length && <p>No saved runs yet.</p>}
      {days.map((group) => {
        const heading = dayHeading(group.items[0]!.created_ms);
        return <section key={group.key} className={styles.day} aria-label={`${heading.name}, ${heading.full}`}>
          <header><h3>{heading.name} <span>{heading.full}</span></h3><span>{group.items.length} {group.items.length === 1 ? "run" : "runs"}</span></header>
          <RowList label="Saved runs">
            {group.items.map((item) => {
              const { title, about, tool } = describe(item);
              return <ListRow key={item.id} title={title} description={about} onOpen={() => onOpen(item.id)}
                meta={[time(item.created_ms), ...(tools.length > 1 ? [tool] : []), item.id.slice(0, 8)]}
                trailing={<RunState execution={item.execution} />}
                details={() => <RunPeek id={item.id} client={client} preview={preview} onOpen={() => onOpen(item.id)} />} />;
            })}
          </RowList>
        </section>;
      })}
      {cursor !== null && <ActionRow><Button busy={busy} onClick={() => void more()}>Load older runs</Button></ActionRow>}
    </section>
  );
}

/** A run's images in and out and the settings it ran with, read when its row is opened. */
function RunPeek({ id, client, preview, onOpen }: { id: string; client: ReturnType<typeof workspaceClient>; preview?: ((asset: Asset) => Promise<Blob>) | undefined; onOpen: () => void }) {
  const [attempt, setAttempt] = useState<Attempt>();
  const [error, setError] = useState<string>();
  useEffect(() => {
    let active = true;
    void client.attempt(id).then((value) => { if (active) setAttempt(value); }).catch((failure) => { if (active) setError(String(failure)); });
    return () => { active = false; };
  }, [client, id]);
  if (error) return <p role="alert">This run's details could not be read.</p>;
  if (!attempt) return <p role="status">Loading details…</p>;
  const action = attempt.declaration.document?.actions.find((value) => value.name === attempt.request.action);
  const properties = object(object(action?.params_schema)?.properties) ?? {};
  const settings = Object.entries(object(attempt.request.params) ?? {}).filter(([, value]) => value !== undefined && value !== null && value !== "");
  const images: { label: string; image: SavedImage }[] = [
    ...Object.entries(attempt.inputs).flatMap(([port, list]) => list.map((image, index) => ({ label: Object.keys(attempt.inputs).length > 1 || list.length > 1 ? `Input · ${port}${list.length > 1 ? ` ${index + 1}` : ""}` : "Input", image }))),
    ...attempt.outputs.map((image, index) => ({ label: attempt.outputs.length > 1 ? `Result ${index + 1}` : "Result", image })),
  ];
  return <div className={styles.peek} data-images={images.length > 0 || undefined}>
    {images.length > 0 && <ul className={styles.images} aria-label="Images">
      {images.map(({ label, image }, index) => <li key={index}><ImageTile asset={image.asset} preview={preview} /><span>{label}</span></li>)}
    </ul>}
    <dl className={styles.settings} aria-label="Settings">
      {settings.length ? settings.map(([key, value]) => <div key={key}>
        <dt>{typeof object(properties[key])?.title === "string" ? String(object(properties[key])!.title) : key}</dt>
        <dd>{typeof value === "string" || typeof value === "number" || typeof value === "boolean" ? String(value) : JSON.stringify(value)}</dd>
      </div>) : <div><dt>Settings</dt><dd>Defaults</dd></div>}
    </dl>
    <ActionRow><Button variant="primary" onClick={onOpen}>Open run</Button></ActionRow>
  </div>;
}
