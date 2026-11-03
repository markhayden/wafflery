import type { DirectoryGrants } from "../../api/directory-grants";
import styles from "./DirectoryAccess.module.css";

export type EffectiveAccess =
  | { state: "reported"; allowed_dirs: string[] }
  | { state: "loading" | "unavailable" };

type Props = { grants: DirectoryGrants | null; effective: EffectiveAccess };
function Folders({ paths, empty }: { paths: string[] | undefined; empty: string }) {
  if (!paths) return <p>Registered folders unavailable.</p>;
  return paths.length ? <ul className={styles.paths}>
    {paths.map((path, index) => <li key={`${index}:${path}`}><code>{path}</code></li>)}
  </ul> : <p>{empty}</p>;
}
export function DirectoryAccess({ grants, effective }: Props) {
  return <section className={styles.access} aria-label="Folder access" aria-busy={effective.state === "loading"}>
    <h3>Folder access</h3>
    <p>The tool’s configuration can restrict the input folders requested by Wafflery.</p>
    <div className={styles.group} role="group" aria-label="Requested input folders">
      <h4>Requested input folders</h4>
      <Folders paths={grants?.requested_dirs} empty="No folders requested." />
    </div>
    <div className={styles.group} role="group" aria-label="Effective input folders">
      <h4>Allowed by this session</h4>
      {effective.state === "reported"
        ? <Folders paths={effective.allowed_dirs} empty="No file paths allowed." />
        : effective.state === "loading"
          ? <p role="status">Checking access…</p>
          : <><p><strong>Unavailable</strong></p><p>Refresh setup to check the current tool session.</p></>}
    </div>
    <div className={styles.group} role="group" aria-label="Result folders">
      <h4>Wafflery may read results from</h4>
      <Folders paths={grants?.evidence_dirs} empty="No result folders granted." />
      <p>These folders do not grant the tool input access.</p>
    </div>
    <p>Change requested folders while Wafflery is stopped. Restart the tool after changing its configuration.</p>
  </section>;
}
