import type {Meta,StoryObj} from "@storybook/react-vite";
import {NativePreservation} from "./NativePreservation";
const attempt={id:"saved",preservation:{contract:"complete",native:"preserved",report:"unsupported"},observations:{native_artifacts:[{direction:"out",index:0,port:"mask",state:"preserved",reason:"Verified native bytes are owned by this library."}]}};
const meta={title:"Workspace/NativePreservation",component:NativePreservation,args:{attempt,retry:async()=>{}}} satisfies Meta<typeof NativePreservation>;
export default meta;type Story=StoryObj<typeof meta>;
export const Preserved:Story={};
export const Preserving:Story={args:{attempt:{...attempt,preservation:{...attempt.preservation,native:"preserving"},observations:{native_artifacts:[{direction:"out",index:0,port:"mask",state:"preserving",reason:"Waiting for native preservation."}]}}}};
export const Failed:Story={args:{attempt:{...attempt,preservation:{...attempt.preservation,native:"failed"},observations:{native_artifacts:[{direction:"out",index:0,port:"mask",state:"failed",reason:"Tool session ended. Retry preservation explicitly; the original action will not be replayed."}]}}}};
export const WorkspaceDependent:Story={args:{attempt:{...attempt,preservation:{...attempt.preservation,native:"workspace-dependent"},observations:{native_artifacts:[{direction:"out",index:0,port:"memo",state:"workspace-dependent",reason:"The tool declares no portable form for this type."}]}}}};
