import type {Action,ActivityKind} from "../actions/model";
import {Picker} from "../../components/Picker";
import {InfoTip} from "../../components/InfoTip";
import {activityChoices,activityKinds,activityTitles,activitySummary,purposeParts,workspaceChoices,workspaceSummary} from "./activities";
import styles from "./ActivityPicker.module.css";
export function ActivityPicker({actions,tool,groups=[],kind,value,onKind,onAction,disabled=false}: {
  actions:Action[]; tool:string; groups?:string[]; kind:ActivityKind; value:string;
  onKind:(kind:ActivityKind)=>void; onAction:(name:string)=>void; disabled?:boolean;
}) {
  const kinds=activityKinds(actions);
  const choices=activityChoices(actions,kind,groups);
  const selected=actions.find(action=>action.name===value);
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
    {selected && selected.readiness.state!=="ready" && <div role="status" className={styles.readiness}>
      <p>{selected.readiness.state==="unavailable" ? "This activity is unavailable." : "This activity needs setup."} Open Tool setup to review requirements.</p>
      <ul>{selected.readiness.requirements.map((requirement,index)=><li key={`${requirement.name}-${index}`}>
        {requirement.name}: {requirement.state}{requirement.setup ? ` — ${requirement.setup}` : ""}
      </li>)}</ul>
    </div>}
    {!disabled && choices.length===0 && <p role="status">No activities are declared in {activityTitles[kind]}.</p>}
  </div>;
}
