import type {Action,ActivityKind} from "../actions/model";
import type {Choice} from "../../components/Picker";
export const activityTitles: Record<ActivityKind,string> = {task:"Playground",operation:"Operations",utility:"Utilities"};
// What each workspace offers, in plain words, naming the tool. The contract defines the three
// kinds the same way for every tool (table-manners 3).
export function workspaceDescription(kind:ActivityKind,tool:string):string {
  return {
    task:`Test out complete jobs. Provide a series of inputs and see how ${tool} works end to end.`,
    operation:`Work one step at a time. Run a single ${tool} operation, see what it makes, and feed the result into the next step.`,
    utility:`Extra ${tool} tools that sit outside complete jobs and single steps.`,
  }[kind];
}
// The closed control stays compact (like the action's summary): how much the workspace holds.
export function workspaceSummary(actions:Action[],kind:ActivityKind):string {
  const count=actions.filter(action=>action.kind===kind).length;
  const noun={task:["complete job","complete jobs"],operation:["operation","operations"],utility:["utility","utilities"]}[kind];
  return `${count} ${count===1?noun[0]:noun[1]}`;
}
export function workspaceChoices(kinds:ActivityKind[],tool:string):Choice[] {
  return kinds.map(id=>({id,title:activityTitles[id],description:workspaceDescription(id,tool)}));
}
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
      ...(action.group?{group:action.group}:{}),description:purposeParts(action.purpose).summary}));
}

// Keep the closed control compact; detailed purpose stays searchable in the chooser and form.
export function activitySummary(action:Action):string {
  if(action.kind==="task")return `${[...(action.images?.length?["Image"]:[]),...(action.locations?.length?["Location"]:[])].join(" + ")||"Parameters"} → Result`;
  const types=(ports:NonNullable<Action["ports"]>["in"])=>ports.map(port=>`${port.type==="image"?"Image":port.type==="location"?"Location":port.type}${port.many?"[]":""}`).join(" + ");
  return `${types(action.ports?.in??[])||"Parameters"} → ${types(action.ports?.out??[])||"Result"}`;
}

// A declared purpose is prose. Its first sentence names the activity in the chooser and under the
// pickers; the rest (when to use it, when not to) waits behind a disclosure. No tool vocabulary is
// parsed: the split is on the first sentence end only.
export function purposeParts(purpose:string|undefined):{summary:string;more:string} {
  const text=(purpose??"").trim();
  const match=/^(.+?[.!?])(?:\s+(.*))?$/s.exec(text);
  return match?{summary:match[1]!,more:(match[2]??"").trim()}:{summary:text,more:""};
}
