import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { ActionForm } from "../../src/features/actions/ActionForm";
import type { Action } from "../../src/features/actions/model";

test("paid forms leave the cap to its dedicated control in simple and JSON modes", () => {
  const action: Action = {kind:"task",name:"priced",title:"Priced",purpose:"Fixture",cost:{may_spend:true},readiness:{state:"ready",requirements:[]},
    params_schema:{type:"object",properties:{label:{type:"string"},max_spend_usd:{type:"number",exclusiveMinimum:0}}}};
  const props = {params:{label:"Keep this",max_spend_usd:0.5},onChange:()=>{},images:{},onImages:()=>{},upload:async()=>{throw Error("unused")},preview:async()=>new Blob()};
  for (const schema of [action.params_schema,{...action.params_schema,allOf:[{type:"object"}]}]) {
    const html = renderToStaticMarkup(<ActionForm {...props} action={{...action,params_schema:schema}} />);
    expect(html).toContain("Keep this");
    expect(html).not.toContain("max_spend_usd");
  }
  const free = renderToStaticMarkup(<ActionForm {...props} action={{...action,cost:{may_spend:false}}} />);
  expect(free).toContain("max_spend_usd");
});
