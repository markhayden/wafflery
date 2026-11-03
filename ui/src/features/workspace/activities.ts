import type {Action,ActivityKind} from "../actions/model";
import type {Choice} from "../../components/Picker";
export const activityTitles: Record<ActivityKind,string> = {task:"Playground",operation:"Operations",utility:"Utilities"};
const kinds: ActivityKind[] = ["task","operation","utility"];
export function activityKinds(actions:Action[]):ActivityKind[] {
  return kinds.filter(kind=>actions.some(action=>action.kind===kind));
}
export function firstActivity(actions:Action[]):Action|undefined {
  const first=activityKinds(actions)[0];
  return actions.find(action=>action.kind===first);
}
export function activityChoices(actions:Action[],kind:ActivityKind,groups:string[]):Choice[] {
  const selected=actions.filter(action=>action.kind===kind);
  // Group vocabulary order is presentation metadata; never infer groups from tool/action names.
  const ordered=[...groups,...selected.flatMap(action=>action.group?[action.group]:[])];
  const groupOrder=[...new Set(ordered)];
  // Ungrouped choices precede headings so they cannot appear to belong to the last group.
  return [...selected.filter(action=>!action.group),
    ...groupOrder.flatMap(group=>selected.filter(action=>action.group===group))].map(action=>({id:action.name,title:action.title,
      ...(action.group?{group:action.group}:{}),description:action.purpose}));
}

// Keep the closed control compact; detailed purpose stays searchable in the chooser and form.
export function activitySummary(action:Action):string {
  if(action.kind==="task")return `${[...(action.images?.length?["Image"]:[]),...(action.locations?.length?["Location"]:[])].join(" + ")||"Parameters"} → Result`;
  const types=(ports:NonNullable<Action["ports"]>["in"])=>ports.map(port=>`${port.type==="image"?"Image":port.type==="location"?"Location":port.type}${port.many?"[]":""}`).join(" + ");
  return `${types(action.ports?.in??[])||"Parameters"} → ${types(action.ports?.out??[])||"Result"}`;
}
