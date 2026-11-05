import type { Meta, StoryObj } from "@storybook/react-vite";
import { CheckVerdict } from "./CheckVerdict";
import sample from "../../../tests/fixtures/sample.png?url";
const frame = { file_sha256: "b".repeat(64), width: 160, height: 100, projection: "flat", topology: "bounded" };
const asset = { id: "a".repeat(32), sha256: "b".repeat(64), size: 10063, width: 160, height: 100, media_type: "image/png" as const };
const outputs = [{ role: "result", label: "Try 3 · result", image: frame, asset }];
const band = { image: frame, kind: "bbox", parts: [{ bbox: [0.3, 0.38, 0.62, 0.62] }], straddles_wrap: false, label: "the band" };
const meta = {
  title: "Workspace/CheckVerdict", component: CheckVerdict,
  render: (args) => <div style={{ maxWidth: 640, padding: "1rem" }}><CheckVerdict {...args} /></div>,
  args: { outputs, load: async () => (await fetch(sample)).blob(),
    result: { status: "completed", verdict: { value: "pass", where: [] }, notes: ["Checked: up to 3 tries within the spend limits.", "Kept try 2 of 3."] } },
} satisfies Meta<typeof CheckVerdict>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Passed: Story = {};
export const StillSeesIt: Story = { args: { result: { status: "completed", verdict: { value: "fail", where: [band] },
  notes: ["Checked: up to 3 tries within the spend limits.", "Kept try 3 of 3. The check still sees the band in 1 place (fail)."] } } };
export const Unsure: Story = { args: { result: { status: "completed", verdict: { value: "unsure", where: [band] },
  notes: ["Kept try 3 of 3. The check still sees the band in 1 place (unsure)."] } } };
export const NotSure: Story = { args: { result: { status: "completed", verdict: { value: "unsure", where: [] },
  notes: ["Checked: up to 3 tries within the spend limits.", "Kept try 1 of 1. The check was not sure and named no place."] } } };
export const NoPlaces: Story = { args: { result: { status: "completed", verdict: { value: "fail", where: [] }, notes: [] } } };
export const NotChecked: Story = { args: { result: { status: "completed", notes: ["Not checked: the checker needs ANTHROPIC_API_KEY."] } } };
export const CouldNotRun: Story = { args: { result: { status: "completed", notes: ["Kept try 1 of 1.", "The check could not run: the checker gave no answer in 120 s."] } } };
