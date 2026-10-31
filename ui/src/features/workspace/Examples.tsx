import {useEffect,useState} from "react";
import {exampleClient,type Example,type ExampleSummary} from "../../api/examples";
import {Button} from "../../components/Button";
export function Examples({client,selectedId,onOpen}:{client:ReturnType<typeof exampleClient>;selectedId?:string|undefined;onOpen:(example:Example)=>void}){
  const [items,setItems]=useState<ExampleSummary[]>([]),[cursor,setCursor]=useState<number|null>(null);
  const [name,setName]=useState(""),[notes,setNotes]=useState("");
  const [busy,setBusy]=useState(false),[error,setError]=useState<string>();
  useEffect(()=>{let active=true;void client.list().then(page=>{if(active){setItems(page.items);setCursor(page.cursor);}}).catch(failure=>{if(active)setError(String(failure));});return()=>{active=false;};},[client]);
  async function save(){
    if(!selectedId)return;setBusy(true);setError(undefined);
    try{await client.save(selectedId,name,notes);const page=await client.list();setItems(page.items);setCursor(page.cursor);setName("");setNotes("");}
    catch(failure){setError(String(failure));}finally{setBusy(false);}
  }
  async function open(id:string){setError(undefined);try{onOpen(await client.open(id));}catch(failure){setError(String(failure));}}
  async function more(){if(cursor===null)return;setBusy(true);try{const page=await client.list(cursor);setItems(previous=>[...previous,...page.items]);setCursor(page.cursor);}catch(failure){setError(String(failure));}finally{setBusy(false);}}
  return <div className="examples">
    {selectedId&&<form onSubmit={event=>{event.preventDefault();void save();}}>
      <h3>Save this run’s inputs</h3><p>Uses the selected saved run. Current draft edits are not included.</p>
      <label>Example name<input required maxLength={120} value={name} onChange={event=>setName(event.target.value)}/></label>
      <label>Notes<textarea maxLength={4000} value={notes} onChange={event=>setNotes(event.target.value)}/></label>
      <Button type="submit" busy={busy}>Save example</Button>
    </form>}
    {error&&<p role="alert">{error}</p>}
    <h3>Saved examples</h3><p>Opening an example fills a new draft. It does not run the tool.</p>
    {!items.length&&<p>No examples saved yet.</p>}
    <ul>{items.map(item=><li key={item.id}><Button onClick={()=>void open(item.id)}>Open {item.name}</Button><p>{item.tool} / {item.action}</p></li>)}</ul>
    {cursor!==null&&<Button busy={busy} onClick={()=>void more()}>Load older examples</Button>}
  </div>;
}
