import type { Meta, StoryObj } from "@storybook/react-vite";
import { ListRow, RowList } from "./ListRow";

const state = (text: string, color: string) => <span style={{ display: "inline-flex", alignItems: "center", gap: "0.5rem", fontSize: "0.8125rem" }}>
  <span style={{ width: "0.5rem", height: "0.5rem", borderRadius: 999, background: color }} />{text}</span>;
const details = () => <dl style={{ margin: 0, display: "grid", gridTemplateColumns: "auto 1fr", gap: "0.25rem 1rem", fontSize: "0.8125rem" }}>
  <dt style={{ color: "var(--color-muted)" }}>Radius</dt><dd style={{ margin: 0 }}>12 px</dd>
  <dt style={{ color: "var(--color-muted)" }}>Sigma</dt><dd style={{ margin: 0 }}>6</dd>
</dl>;

function Runs() {
  return <div style={{ maxWidth: "48rem" }}><RowList label="Saved runs">
    <ListRow title="Profile" description="The band mask of a band profile: the core at each row's half-width about the line, the blend a feather whose sigma follows the row."
      meta={["9:55 PM", "rascal", "40a06a39"]} trailing={state("Completed", "var(--color-ok)")} onOpen={() => undefined} details={details} />
    <ListRow title="Line" description="A line location: a whole column or a path, with its full width in pixels."
      meta={["9:02 PM", "rascal", "bd233cf4"]} trailing={state("Invalid", "var(--color-danger)")} onOpen={() => undefined} details={details} />
  </RowList></div>;
}
function Destinations() {
  return <div style={{ maxWidth: "48rem" }}><RowList label="refine">
    <ListRow title="Feather" description="The blend as a half-Gaussian feather of the core's boundary outward, with one sigma." onOpen={() => undefined} />
    <ListRow title="Subtract · as subtract" description="A mask whose support (core or blend) becomes protect; in the mask's frame." onOpen={() => undefined} />
    <ListRow title="Grow" description="Grows the core by a disc, and further downward for a shadow." disabled onOpen={() => undefined} />
  </RowList></div>;
}
const meta = { title: "Controls/ListRow", component: Runs } satisfies Meta<typeof Runs>;
export default meta;
type Story = StoryObj<typeof meta>;
/** With a state at the right and a chevron that shows details in place. */
export const WithDetails: Story = {};
/** Plain rows to open, each with a chevron; one disabled. */
export const ToOpen: Story = { render: () => <Destinations /> };
/** In a narrow container the rows keep their shape; the state stays on the title's line. */
export const Narrow: Story = { render: () => <div style={{ maxWidth: "22rem" }}><Runs /></div> };
