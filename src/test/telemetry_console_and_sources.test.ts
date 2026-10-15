// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { TopNav } from "../components/layout/TopNav";
import { api } from "../lib/api";
import { render, screen, cleanup } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useToastStore } from "../lib/toast";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

describe("Telemetry Console & Source Workflow UI Hardening", () => {
  beforeEach(() => {
    useToastStore.getState().clearAll();
  });

  describe("TopNav External Terminal Console Launcher", () => {
    it("renders external terminal console launcher button with id and tooltip", () => {
      const html = renderToStaticMarkup(React.createElement(TopNav));

      // Terminal button must exist with id="terminal-console-btn"
      expect(html).toContain('id="terminal-console-btn"');
      expect(html).toContain('aria-label="Open Terminal Console"');

      expect(html).toContain('aria-hidden="true"');
    });

    it("triggers openTerminalConsole when invoked", async () => {
      const spy = vi.spyOn(api, "openTerminalConsole").mockResolvedValue(undefined);
      render(React.createElement(TopNav));
      await userEvent.setup().click(screen.getByRole("button", { name: "Open Terminal Console" }));
      expect(spy).toHaveBeenCalledExactlyOnceWith();
    });
  });

  describe("AddSourceModal & SourcesSettings UI Cleanliness", () => {
    it("offers an SVG upload action for an unsaved workflow draft", async () => {
      const { WorkflowImportButton } = await import("../components/views/settings/WorkflowImportButton");
      render(React.createElement(WorkflowImportButton, { onImport: vi.fn() }));
      const upload = screen.getByRole("button", { name: "Import workflow YAML" });
      expect(upload.querySelector("svg")).toBeTruthy();
      expect(upload.textContent).toBe("");
      expect(screen.getByLabelText("Workflow YAML file")).toBeTruthy();
    });
  });
});
