import {decodedSize,type ImageDimensions} from "../api/image-header";
export type ImageImport = {name:string;dimensions?:ImageDimensions|undefined;index?:number;total?:number};
export function ImageImportProgress({image}:{image:ImageImport}) {
 return <div role="status" style={{overflowWrap:"anywhere"}}>
  <p>{image.name}{image.total&&image.total>1?` · ${image.index} of ${image.total}`:""}</p>
  {image.dimensions&&<p>{image.dimensions.width} × {image.dimensions.height} · {decodedSize(image.dimensions)}</p>}
  <progress aria-label="Importing original image" />
  <p>Importing original… Saving and validating. Preview follows. Original dimensions are kept.</p>
 </div>;
}
