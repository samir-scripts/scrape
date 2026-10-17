import { parseDocument } from "yaml";
import { api } from "./api";

// Registry upload, editor upload and YAML editing share one parser and the same
// canonical backend validator. Failed validation never changes the current draft.
export async function importWorkflowYaml(text: string) {
  const document = parseDocument(text);
  if (document.errors.length) throw new Error(document.errors.map(error => error.message).join("; "));
  return api.importWorkflow(document.toJSON());
}
