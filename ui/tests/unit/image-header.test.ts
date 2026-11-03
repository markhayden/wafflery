import { expect, test } from "bun:test";
import { imageHeader, decodedSize } from "../../src/api/image-header";

function png(width:number,height:number) {
 const bytes=new Uint8Array(24);bytes.set([137,80,78,71,13,10,26,10,0,0,0,13,73,72,68,82]);
 const view=new DataView(bytes.buffer);view.setUint32(16,width);view.setUint32(20,height);return bytes;
}
test("dimensions come from a bounded header read without decoding pixels",async()=>{
 const file=new Blob([png(15000,7500)]);
 expect(await imageHeader(file)).toEqual({width:15000,height:7500});
 expect(decodedSize({width:15000,height:7500})).toBe("429.2 MiB estimated decoded RGBA8");
});
test("JPEG markers skip metadata and find dimensions; a header is provisional",async()=>{
 const bytes=new Uint8Array([255,216,255,224,0,4,1,2,255,194,0,11,8,0,100,0,160,1,1,17,0]);
 expect(await imageHeader(new Blob([bytes]))).toEqual({width:160,height:100});
 for(const invalid of [new Uint8Array(),png(0,20),bytes.slice(0,16),new Uint8Array([255,216,255,224,0,0])])
  expect(await imageHeader(new Blob([invalid]))).toBeUndefined();
});
test("large files read only the header window and do not become decoded images",async()=>{
 const file=new Blob([png(2,1),new Uint8Array(2*1024*1024)]);
 const lengths:number[]=[];const slice=file.slice.bind(file);
 file.slice=(start,end,type)=>{lengths.push((end??file.size)-(start??0));return slice(start,end,type);};
 expect(await imageHeader(file)).toEqual({width:2,height:1});expect(Math.max(...lengths)).toBeLessThanOrEqual(1024*1024);
});
