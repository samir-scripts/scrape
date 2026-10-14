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
    it("ensures source settings and add source files contain no leftover sample SWE tags", async () => {
      const fs = await import("fs/promises");
      const path = await import("path");

      const addSourceModalContent = await fs.readFile(
        path.resolve(process.cwd(), "src/components/views/settings/AddSourceModal.tsx"),
        "utf-8"
      );
      const sourcesSettingsContent = await fs.readFile(
        path.resolve(process.cwd(), "src/components/views/settings/SourcesSettings.tsx"),
        "utf-8"
      );

      // Verify no '[ SAMPLE SWE ]' button or tag in AddSourceModal
      expect(addSourceModalContent).not.toContain("[ SAMPLE SWE ]");
      expect(addSourceModalContent).not.toContain("SAMPLE SWE");

      // Verify no '+ SAMPLE SWE' button in SourcesSettings
      expect(sourcesSettingsContent).not.toContain("+ SAMPLE SWE");

      // Verify SVG upload icon replaces text '[ IMPORT YAML ]'
      expect(addSourceModalContent).not.toContain("[ IMPORT YAML ]");
      expect(addSourceModalContent).toContain('aria-label="Import YAML"');
    });
  });
});
