import type { ReactNode } from "react";
import mark from "../assets/wafflery-mark.svg";

type Props = {
  children: ReactNode;
  toolPicker?: ReactNode;
  controls?: ReactNode;
  connection: string;
  connected?: boolean;
};

/** Shared application chrome; tool state and host controls stay with their owners. */
export function WorkspaceFrame({children, toolPicker, controls, connection, connected = false}: Props) {
  return <>
    <header className="workspace-header">
      <strong className="workspace-brand"><img src={mark} alt="" width="32" height="32" />wafflery</strong>
      {toolPicker && <div className="workspace-tool">{toolPicker}</div>}
      <span className="workspace-connection" role="status">{connection}</span>
      {controls}
    </header>
    <main className="workspace-main" data-connected={connected ? "true" : undefined}>
      {children}
    </main>
  </>;
}
