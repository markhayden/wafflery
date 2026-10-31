import {test,expect} from "./tool-host";
import {readFile} from "node:fs/promises";
import {resolve} from "node:path";
test.use({hostScenario:"slow"});
test("another tab follows a run after a stream reconnect and the launching tab closes",async({page,context,host})=>{
 test.setTimeout(30000);
 await host.open(page);const other=await context.newPage();let connections=0;
 await other.route("**/api/events",async route=>{connections++;if(connections===1)await route.abort();else await route.continue();});
 await host.open(other);
 await expect.poll(()=>connections).toBeGreaterThanOrEqual(2);
 await page.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");await page.getByLabel("Label *",{exact:true}).fill("Survives a closed tab");await page.getByRole("button",{name:"Run",exact:true}).click();
 await expect(other.getByRole("status").filter({hasText:"running"})).toBeVisible();
 await expect(other.getByText("Waiting in the offline test tool",{exact:true})).toBeVisible();
 await page.close();
 await expect(other.getByText("Contract evidence: complete.",{exact:false}).first()).toBeVisible({timeout:15000});
 await expect(other.getByRole("slider")).toBeVisible();
 const calls=await readFile(resolve(host.root,"../tool/calls.ndjson"),"utf8");expect(calls.trim().split("\n")).toHaveLength(1);
 await other.close();
});
