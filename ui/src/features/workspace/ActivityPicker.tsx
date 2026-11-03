import type {Action,ActivityKind} from "../actions/model";
import {Picker} from "../../components/Picker";
import {activityChoices,activityKinds,activityTitles,activitySummary} from "./activities";
import styles from "./ActivityPicker.module.css";
export function ActivityPicker({actions,groups=[],kind,value,onKind,onAction,disabled=false}: {
  actions:Action[]; groups?:string[]; kind:ActivityKind; value:string;
  onKind:(kind:ActivityKind)=>void; onAction:(name:string)=>void; disabled?:boolean;
}) {
  const kinds=activityKinds(actions);
  const choices=activityChoices(actions,kind,groups);
  const selected=actions.find(action=>action.name===value);
  const label=kind==="operation"?"Operation":kind==="utility"?"Utility":"Action";
  return <div className={`${styles.pickers} ${kinds.length>1?styles.multiple:""}`}>
    {kinds.length>1 && <Picker label="Workspace" value={kind} onChange={value=>onKind(value as ActivityKind)}
      disabled={disabled} choices={kinds.map(id=>({id,title:activityTitles[id]}))}/>}
    <Picker label={label} value={value} onChange={onAction} disabled={disabled || choices.length===0} summary={selected?activitySummary(selected):undefined}
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
