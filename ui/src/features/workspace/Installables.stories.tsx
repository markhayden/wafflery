import type { Meta, StoryObj } from "@storybook/react-vite";
import { Installables } from "./Installables";
import type { Installable } from "../actions/model";

const base: Installable = {
  id: "small", title: "OWLv2 + SAM 2.1 Tiny", kind: "model", download_bytes: 749_000_000, installed_bytes: 749_000_000,
  licence: "Apache-2.0", state: "installed", requirements: [{ kind: "file", name: "free disk", state: "present" }],
  notes: "The default.", last_outcome: null, "x-rascal": { lab: { remove: 0.548, best_for: "small and fast" } },
};
const large: Installable = {
  ...base, id: "sam3", title: "SAM 3", download_bytes: 3_400_000_000, installed_bytes: 3_400_000_000,
  licence: "SAM License", state: "not_installed", notes: "Best removal score in the lab.",
  "x-rascal": { lab: { remove: 0.777, best_for: "whole objects and holes" } },
};
const meta = {
  title: "Workspace/Installables", component: Installables,
  args: {
    installables: [base, large],
    onInstall: async () => undefined, onUninstall: async () => undefined, onRefresh: async () => undefined,
  },
} satisfies Meta<typeof Installables>;
export default meta;
type Story = StoryObj<typeof meta>;
export const InstalledAndNot: Story = {};
export const Installing: Story = { args: { installables: [base, { ...large, state: "installing", progress: 0.4 }] } };
export const InstallingWithoutProgress: Story = { args: { installables: [{ ...large, state: "installing" }] } };
export const Failed: Story = { args: { installables: [{ ...large, last_outcome: {
  action: "install", state: "failed", at: "2026-11-09T12:00:00Z",
  error: { code: "failed", message: "sam3.vision.onnx.data: sha256 did not match" },
} }] } };
export const Cancelled: Story = { args: { installables: [{ ...large, last_outcome: {
  action: "install", state: "cancelled", at: "2026-11-09T12:00:00Z", error: null,
} }] } };
export const Unavailable: Story = { args: { installables: [{ ...large, state: "unavailable", notes: "Not built for linux-arm64." }] } };
export const RefusedControl: Story = { args: { onInstall: async () => { throw new Error("The tool cannot install this yet; see its requirements"); } } };
