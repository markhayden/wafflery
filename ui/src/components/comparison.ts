export type Coordinates = {width:number;height:number;projection:string;topology:string};
export function canWipe(left:Coordinates|undefined,right:Coordinates|undefined):boolean {
 return !!left && !!right && left.width===right.width && left.height===right.height && left.projection===right.projection && left.topology===right.topology;
}
