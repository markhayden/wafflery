import type {LocalApi} from "./client";
export type EventNotice={session:string;sequence:number;kind:string};
export function eventNotice(line:string):EventNotice {
 const value:unknown=JSON.parse(line);
 if(!value||typeof value!=="object"||!("session" in value)||typeof value.session!=="string"||!("sequence" in value)||!Number.isSafeInteger(value.sequence)||!("kind" in value)||typeof value.kind!=="string")throw new Error("Invalid workspace event.");
 return value as EventNotice;
}
function pause(signal:AbortSignal):Promise<void>{return new Promise(resolve=>{const finish=()=>{clearTimeout(timer);signal.removeEventListener("abort",finish);resolve();};const timer=setTimeout(finish,1000);signal.addEventListener("abort",finish,{once:true});if(signal.aborted)finish();});}
export async function watchWorkspace(api:LocalApi,session:string,onChange:()=>Promise<void>,connection:(message:string)=>void,signal:AbortSignal):Promise<void> {
 while(!signal.aborted){
  const attempt=new AbortController();const abort=()=>attempt.abort();signal.addEventListener("abort",abort,{once:true});
  let watchdog:ReturnType<typeof setTimeout>|undefined;
  try{
   const response=await api.request("/api/events",{signal:attempt.signal});const reader=response.body?.getReader();if(!reader)throw new Error("Workspace stream unavailable.");
   const decoder=new TextDecoder();let buffer="";connection("");
   try{while(!signal.aborted){
    clearTimeout(watchdog);watchdog=setTimeout(()=>attempt.abort(),35000);
    const {value,done}=await reader.read();if(done)break;buffer+=decoder.decode(value,{stream:true});
    let end:number;while((end=buffer.indexOf("\n"))>=0){if(end>16384)throw new Error("Workspace event exceeds its limit.");const event=eventNotice(buffer.slice(0,end));buffer=buffer.slice(end+1);
     if(event.session!==session){connection("This host session changed. Run wafflehouse to reopen it.");return;}
     if(event.kind!=="heartbeat")await onChange();
    }
    if(buffer.length>16384)throw new Error("Workspace event exceeds its limit.");
   }}finally{await reader.cancel().catch(()=>{});reader.releaseLock();}
   if(!signal.aborted)connection("Reconnecting to the workspace…");
  }catch(error){if(!signal.aborted){const message=error instanceof Error?error.message:"Workspace stream disconnected.";connection(message.includes("session ended")?message:"Reconnecting to the workspace…");if(message.includes("session ended"))return;}}
  finally{clearTimeout(watchdog);signal.removeEventListener("abort",abort);attempt.abort();}
  if(!signal.aborted)await pause(signal);
 }
}
