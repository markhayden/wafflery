import type { LocalApi } from "./client";
import type { Attempt, Submission } from "./workspace";
export type Example = {
  id: string;
  sequence: number;
  name: string;
  notes: string;
  request: Submission;
  inputs: Attempt["inputs"];
  declaration: Attempt["declaration"];
  input_artifacts: import("../features/actions/model").Schema[];
  workspace_dependent: boolean;
};
export type ExampleSummary = {
  id: string;
  sequence: number;
  name: string;
  tool: string;
  action: string;
  workspace_dependent: boolean;
};
export function exampleClient(api: LocalApi) {
  async function read(response: Response): Promise<Example> {
    const value: unknown = await response.json();
    if (
      !value ||
      typeof value !== "object" ||
      !("id" in value) ||
      typeof value.id !== "string" ||
      !("request" in value) ||
      !("inputs" in value) ||
      !("name" in value) ||
      typeof value.name !== "string" ||
      !("workspace_dependent" in value) || typeof value.workspace_dependent!=="boolean" ||
      !("input_artifacts" in value) || !Array.isArray(value.input_artifacts) ||
      !("declaration" in value) || !value.declaration || typeof value.declaration!=="object"
    )
      throw new Error("Example could not be read.");
    return value as Example;
  }
  return {
    async list(
      cursor?: number,
    ): Promise<{ items: ExampleSummary[]; cursor: number | null }> {
      const value = (await (
        await api.request(
          `/api/examples${cursor === undefined ? "" : `?cursor=${cursor}`}`,
        )
      ).json()) as { items: ExampleSummary[]; cursor: number | null };
      if (
        !Array.isArray(value.items) ||
        !value.items.every(
          (item) =>
            typeof item.id === "string" && typeof item.name === "string" && typeof item.workspace_dependent==="boolean",
        )
      )
        throw new Error("Examples could not be read.");
      return value;
    },
    open: async (id: string) =>
      read(await api.request(`/api/examples/${encodeURIComponent(id)}`)),
    save: async (attempt_id: string, name: string, notes: string, allow_workspace_dependent=false) =>
      read(
        await api.request("/api/examples", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ attempt_id, name, notes, allow_workspace_dependent }),
        }),
      ),
  };
}
