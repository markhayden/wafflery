import { Button } from "./components/Button";

export function App() {
  return (
    <>
      <header className="workspace-header">
        <strong>wafflehouse</strong>
        <span>Local workspace</span>
      </header>
      <main className="workspace-main">
        <h1>Your workshop</h1>
        <p className="workspace-intro">Try a tool. Inspect the result. Take it into your pipeline.</p>
        <section className="workspace-empty" aria-labelledby="setup-heading">
          <h2 id="setup-heading">No tools connected yet</h2>
          <p id="connection-help">This foundation build is ready for interface development. Tool connections and saved runs will arrive in the next milestones.</p>
        <div><Button disabled aria-describedby="connection-help">Connect a tool</Button></div>
        </section>
      </main>
    </>
  );
}
