import {useEffect,useState} from "react";
import type {LocalApi} from "../api/client";
import {Button} from "./Button";
import {Drawer} from "./Drawer";
type Status={phase:"running"|"stopping"|"waiting"|"stopped";active_attempts:number};
export function Shutdown({api}:{api:LocalApi}){
  const [opener,setOpener]=useState<HTMLElement|null>(null);
  const [status,setStatus]=useState<Status>(),[notice,setNotice]=useState("");
  const [busy,setBusy]=useState(false),[connectionLost,setConnectionLost]=useState(false),[retry,setRetry]=useState(0);
  useEffect(()=>{
    if(!opener)return;let active=true;let timer:ReturnType<typeof setTimeout>;
    async function refresh(){
      try{const value=await(await api.request("/api/host/shutdown")).json() as Status;if(active){setStatus(value);setConnectionLost(false);setNotice("");timer=setTimeout(()=>void refresh(),500);}}
      catch(failure){if(active){setConnectionLost(true);setNotice(status&&status.phase!=="running"?"Shutdown requested. The connection is unavailable; the final host state has not been confirmed.":failure instanceof Error?failure.message:"The host is unavailable.");timer=setTimeout(()=>void refresh(),1000);}}
    }
    void refresh();return()=>{active=false;clearTimeout(timer);};
  },[api,opener,retry,status?.phase]);
  async function request(force=false){
    setBusy(true);setNotice("");
    try{const response=await api.request(force?"/api/host/force-quit":"/api/host/quit",{method:"POST",headers:{"Content-Type":"application/json"},body:force?"{}":JSON.stringify({confirmed:true})});setStatus(await response.json() as Status);}
    catch(failure){setNotice(failure instanceof Error?failure.message:"Shutdown could not be requested.");}finally{setBusy(false);}
  }
  return <>
    <Button onClick={event=>{setOpener(event.currentTarget);setNotice("");}}>Quit workspace</Button>
    <Drawer open={opener!==null} opener={opener} onClose={()=>setOpener(null)} title="Quit workspace" description="The local server and its owned tools will stop. Saved evidence remains in your library.">
      <div className="inspector">
        {status?.phase==="running"&&<><p>{status.active_attempts?`${status.active_attempts} active or unconfirmed attempts will receive a cancellation request.`:"No active attempts are currently recorded."}</p><p>Closing a browser tab leaves the workspace running. Confirm below to stop it.</p><Button onClick={()=>setOpener(null)}>Keep running</Button><Button busy={busy} onClick={()=>void request()}>Confirm quit</Button></>}
        {status?.phase==="stopping"&&<p role="status">Stopping tools and preserving saved evidence…</p>}
        {status?.phase==="waiting"&&<><p role="status">Waiting for tools or evidence preservation to finish. The host still owns those processes.</p><p>Force quit may lose unfinished output. It only stops tools started by this workspace.</p><Button busy={busy} onClick={()=>void request(true)}>Force quit owned tools</Button></>}
        {(connectionLost||status?.phase==="stopped")&&<p>Run wafflehouse again to reopen the workspace.</p>}
        {notice&&<p role="status">{notice}</p>}
        {connectionLost&&<Button onClick={()=>setRetry(value=>value+1)}>Retry connection</Button>}
      </div>
    </Drawer>
  </>;
}
