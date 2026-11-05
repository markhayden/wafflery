import {ListRow,RowList} from "../../components/ListRow";
import type {Destination} from "./artifact-transfer";
import styles from "./SendDestinations.module.css";

/** Where an output can go next, as rows: each operation that takes it, with what it does, grouped
 * the way the tool groups its operations. Choosing one fills a new draft; nothing runs. */
export function SendDestinations({destinations,groups,onChoose,disabled=false}:{destinations:Destination[];groups:Record<string,string[]>;onChoose:(destination:Destination)=>void;disabled?:boolean}){
 if(!destinations.length)return <p role="status">No installed operation takes this kind of output, so there's nowhere to send it.</p>;
 const tools=[...new Set(destinations.map(item=>item.toolName))];
 const sections=tools.flatMap(tool=>{
  const items=destinations.filter(item=>item.toolName===tool);
  const order=[...(groups[tool]??[]),...items.flatMap(item=>item.group?[item.group]:[])].filter((group,index,all)=>all.indexOf(group)===index);
  const ungrouped=items.filter(item=>!item.group);
  return [...(ungrouped.length?[{key:`${tool}/`,heading:tools.length>1?tool:undefined,items:ungrouped}]:[]),
   ...order.map(group=>({key:`${tool}/${group}`,heading:tools.length>1?`${tool} · ${group}`:group,items:items.filter(item=>item.group===group)})).filter(section=>section.items.length)];
 });
 return <div className={styles.sections}>
  {sections.map(section=><section key={section.key} className={styles.section} aria-label={section.heading??"Operations"}>
   {section.heading&&<h3>{section.heading}</h3>}
   <RowList label={section.heading??"Operations"}>
    {section.items.map(item=><ListRow key={`${item.tool}/${item.action}/${item.port}`} title={item.title} description={item.description}
     disabled={disabled} onOpen={()=>onChoose(item)}/>)}
   </RowList>
  </section>)}
 </div>;
}
