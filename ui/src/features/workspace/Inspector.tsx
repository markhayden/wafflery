import {useState} from "react";
import type {Attempt} from "../../api/workspace";
import {Button} from "../../components/Button";
export function Inspector({attempt}:{attempt:Attempt}) {
 const [view,setView]=useState<"inputs"|"result"|"call"|"journal">("inputs");
 const data=view==="inputs"?{parameters:attempt.request.params,images:attempt.inputs}:view==="result"?attempt.raw_response:view==="call"?attempt.wire:attempt.journal;
 return <div className="inspector"><nav aria-label="Run evidence"><Button aria-pressed={view==="inputs"} onClick={()=>setView("inputs")}>Saved inputs</Button><Button aria-pressed={view==="result"} onClick={()=>setView("result")}>Raw result</Button><Button aria-pressed={view==="call"} onClick={()=>setView("call")}>Recorded call</Button><Button aria-pressed={view==="journal"} onClick={()=>setView("journal")}>Journal</Button></nav>
 <p>Tool outcome: {attempt.execution}. Contract evidence: {attempt.preservation.contract}.</p>
 <pre aria-label="Saved evidence">{JSON.stringify(data,null,2)??"No evidence available yet."}</pre>
 <p>Native dependencies and tool reports are not preserved by this foundation.</p>
 {attempt.notices.map((notice,index)=><p key={index}>{notice}</p>)}
 </div>;
}
