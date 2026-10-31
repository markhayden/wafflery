import { useEffect, useState } from "react";
import { workspaceClient, type AttemptSummary } from "../../api/workspace";
import { Button } from "../../components/Button";

export function History({client,onOpen}:{client:ReturnType<typeof workspaceClient>;onOpen:(id:string)=>void}) {
  const [items,setItems]=useState<AttemptSummary[]>([]);
  const [cursor,setCursor]=useState<number|null>(null);
  const [error,setError]=useState<string>();
  const [busy,setBusy]=useState(false);
  useEffect(()=>{
    let active=true;
    setBusy(true);
    void client.history().then(page=>{if(active){setItems(page.items);setCursor(page.cursor);}})
      .catch(failure=>{if(active)setError(String(failure));}).finally(()=>{if(active)setBusy(false);});
    return()=>{active=false;};
  },[client]);
  async function more(){
    if(cursor===null||busy)return;
    setBusy(true);setError(undefined);
    try{const page=await client.history(cursor);setItems(previous=>[...previous,...page.items]);setCursor(page.cursor);}
    catch(failure){setError(String(failure));}finally{setBusy(false);}
  }
  return <section aria-label="Saved history">
    {error&&<p role="alert">{error}</p>}
    {!busy&&!items.length&&<p>No saved attempts yet.</p>}
    <ul>{items.map(item=><li key={item.id}><Button onClick={()=>onOpen(item.id)}>
      {item.tool} / {item.action} · {item.execution} · {new Date(item.created_ms).toLocaleString()} · {item.id.slice(0,8)}
    </Button></li>)}</ul>
    {cursor!==null&&<Button busy={busy} onClick={()=>void more()}>Load older runs</Button>}
  </section>;
}
