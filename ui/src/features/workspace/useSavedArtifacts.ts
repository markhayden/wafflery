import {useEffect,useState} from "react";
import type {LocalApi} from "../../api/client";
import {workspaceClient,type Snapshot} from "../../api/workspace";
import {savedSources,type ArtifactSource} from "./artifact-transfer";
export function useSavedArtifacts(api:LocalApi,snapshot:Snapshot,revision=""){
 const key=JSON.stringify([snapshot.attempts,revision]);
 const [pages,setPages]=useState(1),[sources,setSources]=useState<ArtifactSource[]>([]),[more,setMore]=useState(false),[error,setError]=useState<string>(),[loading,setLoading]=useState(false);
 useEffect(()=>{let active=true;setLoading(true);const client=workspaceClient(api);void(async()=>{
 let cursor:number|undefined;const values:ArtifactSource[]=[];let hasMore=false;
 for(let page=0;page<pages;page++){
 const history=await client.history(cursor);const reads=await Promise.allSettled(history.items.map(item=>client.attempt(item.id)));
 const attempts=reads.flatMap(read=>read.status==="fulfilled"?[read.value]:[]);
 values.push(...attempts.flatMap(savedSources));hasMore=history.cursor!==null;if(!hasMore)break;cursor=history.cursor!;
 }
 if(active){setSources(values);setMore(hasMore);setError(undefined);}
 })().catch(failure=>{if(active){setSources([]);setError(String(failure));}}).finally(()=>{if(active)setLoading(false);});return()=>{active=false;};},[api,key,pages]);
 return {sources,more,error,loading,loadMore:()=>setPages(value=>value+1)};
}
