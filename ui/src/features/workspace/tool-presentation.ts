import type {Choice} from "../../components/Picker";
import type {RegisteredTool} from "../../api/workspace";
const presentation = new Map([
  ["rascal",{icon:new URL("../../assets/tools/rascal.webp?no-inline",import.meta.url).href,
    description:"Fully automated mask generation for precise image edits. Describe what to change and rascal finds the region, builds a high-quality mask and protects everything else, across a wide range of use cases."}],
  ["squint",{icon:new URL("../../assets/tools/squint.webp?no-inline",import.meta.url).href,
    description:"Image evaluation you can trust. Checks an image against the brief and references it was made from, and shows the evidence behind every verdict."}],
]);
export function toolChoice(tool:Pick<RegisteredTool,"id"|"declaration"|"error">):Choice {
  const declared=tool.declaration?.document.tool;
  const name=declared?.name??tool.id;
  const known=declared?presentation.get(declared.name):undefined;
  return {id:tool.id,title:name,meta:`Version ${declared?.version??"unknown"}`,
    description:tool.error??known?.description??declared?.purpose??"Tool setup is unavailable.",
    ...(known?{icon:known.icon}:{})};
}
