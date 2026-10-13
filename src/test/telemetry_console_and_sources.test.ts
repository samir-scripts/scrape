import { describe, it, expect, vi, beforeEach } from "vitest";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { TopNav } from "../components/layout/TopNav";
import { api } from "../lib/api";
import { useToastStore } from "../lib/toast";

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

      // Terminal SVG icon has terminal prompt glyphs (> and _)
      expect(html).toContain('<rect x="1.5" y="2.5" width="13" height="11"');
      expect(html).toContain('<path d="M4.5 6l2.5 2-2.5 2"');
      expect(html).toContain('<line x1="8.5" y1="10" x2="11.5" y2="10"');
    });

    it("triggers openTerminalConsole when invoked", async () => {
      const spy = vi.spyOn(api, "openTerminalConsole");
      await api.openTerminalConsole();
      expect(spy).toHaveBeenCalledTimes(1);
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
