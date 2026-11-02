import type { Meta, StoryObj } from "@storybook/react-vite";
import { DirectoryAccess } from "./DirectoryAccess";
const grants = {
  requested_dirs: ["/work/library/execution", "/work/reference images"],
  evidence_dirs: ["/work/tool/results"],
};
const meta = {
  title: "Workspace/DirectoryAccess", component: DirectoryAccess,
  args: { grants, effective: {state: "reported", allowed_dirs: grants.requested_dirs} },
} satisfies Meta<typeof DirectoryAccess>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Reported: Story = {};
export const Narrowed: Story = {args: {effective: {state:"reported", allowed_dirs:["/work/reference images/approved"]}}};
export const NoAccess: Story = {args: {grants:{requested_dirs:[],evidence_dirs:[]}, effective:{state:"reported",allowed_dirs:[]}}};
export const NotReported: Story = {args:{effective:{state:"not_reported"}}};
export const Loading: Story = {args:{effective:{state:"loading"}}};
export const Unavailable: Story = {args:{grants:null,effective:{state:"unavailable"}}};
const longPath = `/work/${"a-long-project-folder-".repeat(8)}/<literal> $(name)`;
export const LongPaths: Story = {args:{grants:{requested_dirs:[longPath], evidence_dirs:["/work/tool/results"]},effective:{state:"reported",allowed_dirs:[longPath]}}};
