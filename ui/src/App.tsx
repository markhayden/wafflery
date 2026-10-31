import { Button } from "./components/Button";
import { useEffect, useState } from "react";
import { connect } from "./api/session";

export function App() {
  const [connection, setConnection] = useState("Connecting…");
  useEffect(() => {
    let active = true;
    void connect().then(session => {
      if (active) setConnection(session ? "Connected locally" : "Run wafflehouse in your terminal to open this workspace.");
    }).catch((error: unknown) => {
      if (active) setConnection(error instanceof Error ? error.message : "Unable to connect to the workspace.");
    });
    return () => { active = false; };
  }, []);
  return (
    <>
      <header className="workspace-header">
        <strong>wafflehouse</strong>
        <span>Local workspace</span>
      </header>
      <main className="workspace-main">
        <h1>Your workshop</h1>
        <p className="workspace-intro">Try a tool. Inspect the result. Take it into your pipeline.</p>
        <p role="status">{connection}</p>
        <section className="workspace-empty" aria-labelledby="setup-heading">
          <h2 id="setup-heading">No tools connected yet</h2>
          <p id="connection-help">This foundation build is ready for interface development. Tool connections and saved runs will arrive in the next milestones.</p>
        <div><Button disabled aria-describedby="connection-help">Connect a tool</Button></div>
        </section>
      </main>
    </>
  );
}
