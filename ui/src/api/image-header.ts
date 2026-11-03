/** Provisional header feedback only. The host still validates the complete encoded image. */
export type ImageDimensions = {width:number;height:number};
export async function imageHeader(file:Blob):Promise<ImageDimensions|undefined> {
 const bytes=new Uint8Array(await file.slice(0,Math.min(file.size,1024*1024)).arrayBuffer());
 const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
 const dimensions=(width:number,height:number)=>width>0&&height>0?{width,height}:undefined;
 if(bytes.length>=24&&[137,80,78,71,13,10,26,10].every((byte,index)=>bytes[index]===byte)
  &&view.getUint32(8)===13&&view.getUint32(12)===0x49484452)
  return dimensions(view.getUint32(16),view.getUint32(20));
 if(bytes[0]!==255||bytes[1]!==216)return;
 let offset=2;
 while(offset+3<bytes.length){
  if(bytes[offset++]!==255)return;
  while(bytes[offset]===255)offset++;
  const marker=bytes[offset++];if(marker===undefined||marker===0xda||marker===0xd9)return;
  if(marker===1||(marker>=0xd0&&marker<=0xd7))continue;
  if(offset+2>bytes.length)return;
  const length=view.getUint16(offset);if(length<2||offset+length>bytes.length)return;
  if([0xc0,0xc1,0xc2,0xc3,0xc5,0xc6,0xc7,0xc9,0xca,0xcb,0xcd,0xce,0xcf].includes(marker)){
   if(length<8)return;
   return dimensions(view.getUint16(offset+5),view.getUint16(offset+3));
  }
  offset+=length;
 }
}
export function decodedSize(size:ImageDimensions):string {
 return `${(size.width*size.height*4/1024/1024).toFixed(1)} MiB estimated decoded RGBA8`;
}
