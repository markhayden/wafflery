import styles from "./Library.module.css";
import { useEffect, useState } from "react";
import {
  exampleClient,
  type Example,
  type ExampleSummary,
} from "../../api/examples";
import { Button } from "../../components/Button";
export function Examples({
  client,
  selectedId,
  onOpen,
}: {
  client: ReturnType<typeof exampleClient>;
  selectedId?: string | undefined;
  onOpen: (example: Example) => void|Promise<void>;
}) {
  const [items, setItems] = useState<ExampleSummary[]>([]),
    [cursor, setCursor] = useState<number | null>(null);
  const [name, setName] = useState(""),
    [notes, setNotes] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [allowDependent,setAllowDependent]=useState(false);
  const [listError, setListError] = useState<string>();
  const [saveError, setSaveError] = useState<string>();
  const [saveNotice, setSaveNotice] = useState<string>();
  const [openError, setOpenError] = useState<{id:string; message:string}>();
  useEffect(() => {
    let active = true;
    setLoading(true);
    setListError(undefined);
    void client
      .list()
      .then((page) => {
        if (active) {
          setItems(page.items);
          setCursor(page.cursor);
        }
      })
      .catch((failure) => {
        if (active) setListError(String(failure));
      })
      .finally(() => { if (active) setLoading(false); });
    return () => {
      active = false;
    };
  }, [client]);
  useEffect(()=>setAllowDependent(false),[selectedId]);
  async function save() {
    if (!selectedId) return;
    setBusy(true);
    setSaveError(undefined);
    setSaveNotice(undefined);
    let saved:Example;
    try {
      saved=await client.save(selectedId, name, notes,allowDependent);
    } catch (failure) {
      setSaveError(String(failure));
      setBusy(false);
      return;
    }
    // Saving succeeded even if the following list refresh is unavailable.
    setName("");
    setNotes("");setAllowDependent(false);
    try {
      const page = await client.list();
      setItems(page.items);
      setCursor(page.cursor);
      setListError(undefined);
      setSaveNotice(saved.workspace_dependent?"Saved as workspace-dependent.":"Example saved.");
    } catch (failure) {
      setListError(String(failure));
      setSaveNotice(`${saved.workspace_dependent?"Workspace-dependent example saved.":"Example saved."} The list could not be refreshed.`);
    } finally {
      setBusy(false);
    }
  }
  async function open(id: string) {
    setOpenError(undefined);
    try {
      await onOpen(await client.open(id));
    } catch (failure) {
      setOpenError({id,message:String(failure)});
    }
  }
  async function more() {
    if (cursor === null) return;
    setListError(undefined);
    setBusy(true);
    try {
      const page = await client.list(cursor);
      setItems((previous) => [...previous, ...page.items]);
      setCursor(page.cursor);
    } catch (failure) {
      setListError(String(failure));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className={styles.library} aria-busy={loading}>
      <h3>Saved examples</h3>
      <p>Opening an example fills a new draft. It does not run the tool.</p>
      {loading && <p role="status">Loading examples…</p>}
      {!loading && !listError && !items.length && <div className={styles.empty}>
        <p>No examples saved yet.</p><p>Run a tool, then save its inputs here to try them again.</p>
      </div>}
      {!!items.length && <ul>
        {items.map((item) => (
          <li key={item.id}>
            <Button className={styles.entry} aria-label={`Open ${item.name}`} onClick={() => void open(item.id)}>
              <span><strong>{item.name}</strong><small>{item.tool} / {item.action}{item.workspace_dependent?" · Workspace-dependent":""}</small></span>
              <span aria-hidden="true">→</span>
            </Button>
            {openError?.id === item.id && <p role="alert">{openError.message}</p>}
          </li>
        ))}
      </ul>}
      {listError && <p role="alert">{listError}</p>}
      {cursor !== null && (
        <Button busy={busy} onClick={() => void more()}>
          Load older examples
        </Button>
      )}
      {selectedId && (
        <form className={styles.saveForm}
          onSubmit={(event) => {
            event.preventDefault();
            void save();
          }}
        >
          <h3>Save this run’s inputs</h3>
          <p>
            Uses the selected saved run. Current draft edits are not included.
          </p>
          <label>
            Example name
            <input
              required
              maxLength={120}
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
          </label>
          <label>
            Notes
            <textarea
              maxLength={4000}
              value={notes}
              onChange={(event) => setNotes(event.target.value)}
            />
          </label>
          <label><input type="checkbox" checked={allowDependent} onChange={event=>setAllowDependent(event.target.checked)}/> Allow a workspace-dependent example if native inputs are not preserved</label>
          {saveError && <p role="alert">{saveError}</p>}
          {saveNotice && <p role="status">{saveNotice}</p>}
          <Button type="submit" busy={busy}>
            Save example
          </Button>
        </form>
      )}
    </div>
  );
}
