import type {Action,ActivityKind,Readiness} from "../actions/model";
import {Picker} from "../../components/Picker";
import {InfoTip} from "../../components/InfoTip";
import {activityChoices,activityKinds,activityTitles,activitySummary,purposeParts,workspaceChoices,workspaceSummary} from "./activities";
import { blocking } from "./checks";
import styles from "./ActivityPicker.module.css";
export function ActivityPicker({actions,tool,groups=[],kind,value,onKind,onAction,disabled=false,readiness}: {
  actions:Action[]; tool:string; groups?:string[]; kind:ActivityKind; value:string;
  /** The selected activity's readiness for the current inputs; the tool's own when not given. */
  readiness?:Readiness|undefined;
  onKind:(kind:ActivityKind)=>void; onAction:(name:string)=>void; disabled?:boolean;
}) {
  const kinds=activityKinds(actions);
  const choices=activityChoices(actions,kind,groups);
  const selected=actions.find(action=>action.name===value);
  const ready=readiness??selected?.readiness;
  const label=kind==="operation"?"Operation":kind==="utility"?"Utility":"Action";
  const about=selected?purposeParts(selected.purpose):undefined;
  return <div className={`${styles.pickers} ${kinds.length>1?styles.multiple:""}`}>
    {kinds.length>1 && <Picker label="Workspace" value={kind} onChange={value=>onKind(value as ActivityKind)} summary={workspaceSummary(actions,kind)}
      disabled={disabled} choices={workspaceChoices(kinds,tool)}/>}
    {/* The selected activity's card says what it does (its first sentence) under its name, with its
        types as a tag; the rest of its purpose is one ⓘ away, beside the label, like any field. */}
    <Picker label={label} value={value} onChange={onAction} disabled={disabled || choices.length===0}
      summary={about?.summary} tag={selected?activitySummary(selected):undefined} oneLine
      labelExtra={selected && about?.more ? <InfoTip label={selected.title}>{about.more}</InfoTip> : undefined}
      choices={choices}/>
    {selected && ready && ready.state!=="ready" && <div role="status" className={styles.readiness}>
      <p>{ready.state==="unavailable" ? "This activity is unavailable." : "This activity needs setup."} Open Tool setup to review requirements.</p>
      <ul>{blocking(ready.requirements).filter(requirement=>requirement.state!=="present"&&requirement.state!=="valid").map((requirement,index)=><li key={`${requirement.name}-${index}`}>
        {requirement.name}: {requirement.state}{requirement.setup ? ` — ${requirement.setup}` : ""}
      </li>)}</ul>
    </div>}
    {!disabled && choices.length===0 && <p role="status">No activities are declared in {activityTitles[kind]}.</p>}
  </div>;
}
