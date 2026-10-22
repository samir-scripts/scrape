// @vitest-environment jsdom
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { UserEvent } from "@testing-library/user-event";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";
import { api, SAMPLE_SWE_SOURCE } from "../lib/api";
import type { ExtractorTestResult } from "../bindings/ExtractorTestResult";
import type { BrowserRuntimeStatus } from "../bindings/BrowserRuntimeStatus";
import type { BrowserSessionSummary } from "../bindings/BrowserSessionSummary";
import type { ScriptEnvironmentStatus } from "../bindings/ScriptEnvironmentStatus";
import type { Source } from "../bindings/Source";
import type { WorkflowSpec } from "../bindings/WorkflowSpec";

async function openExtractorSettings(user: UserEvent) {
  await user.click(screen.getByRole("button", { name: "Continue to Criteria" }));
  await user.click(screen.getByRole("button", { name: "Continue to Sources" }));
  await user.click(screen.getByRole("button", { name: "Advanced" }));
}
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
  };
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const sampleSelectorWorkflow: WorkflowSpec = {
  version: 6,
  metadata: { name: "Custom Extractor Workflow", enabled: true },
  sources: [
    {
      id: "source-1",
      label: "Custom Selector Source",
      enabled: true,
      extractor: {
        kind: "selector",
        mode: "static",
        url: "https://example.com/careers",
        job_container: "div.job-item",
        title_selector: "h2.job-title",
        url_selector: "a.apply-link",
      },
    },
  ],
};

const sampleScriptWorkflow: WorkflowSpec = {
  version: 6,
  metadata: { name: "Script Workflow", enabled: false },
  sources: [
    {
      id: "src-script-1",
      label: "Script Runner Source",
      enabled: false,
      extractor: {
        kind: "selector",
        mode: "script",
        url: "https://example.com/script-jobs",
        script: {
          runtime: "python",
          interpreter: "/usr/bin/python3",
          path: "./scripts/scrape_jobs.py",
          args: ["--mode", "full"],
          timeout_s: 30,
        },
      },
    },
  ],
};

const sampleScriptSource: Source = {
  ...SAMPLE_SWE_SOURCE,
  id: "src-script-1",
  name: "Script Source",
  kind: "selector",
  needs_review: true,
  script_trusted: false,
  script_sha256: "abc123sha256fingerprint",
  config: {
    __workflow_spec: sampleScriptWorkflow,
  },
};

describe("Workflow Editor — Custom Extractor, Browser Sessions, & Script Trust (Section 6E)", () => {
  it("allows switching extraction mode and entering CSS and XPath locators", async () => {
    const user = userEvent.setup({ delay: null });

    render(
      <AddSourceModal
        isOpen
        initialSpec={sampleSelectorWorkflow}
        onClose={vi.fn()}
        onSourceSaved={vi.fn()}
      />
    );

    // Advanced mode must be selected to edit extractor details
    await openExtractorSettings(user);

    // Mode selector is visible
    expect(screen.getByRole("button", { name: /Extraction mode/i })).toBeTruthy();
    expect(screen.getByText(/Static HTML · CSS & XPath/i)).toBeTruthy();

    // Required locators inputs are rendered with initial values
    const containerInput = screen.getByRole("textbox", { name: /Job container locator/i });
    expect((containerInput as HTMLInputElement).value).toBe("div.job-item");

    // Enter XPath locators
    await user.clear(containerInput);
    await user.type(containerInput, "xpath://article[[@data-job-id]");
    expect(containerInput).toHaveProperty("value", "xpath://article[@data-job-id]");

    const titleInput = screen.getByRole("textbox", { name: /Title locator/i });
    await user.clear(titleInput);
    await user.type(titleInput, "xpath:.//h3[[contains(@class, 'title')]");
    expect(titleInput).toHaveProperty("value", "xpath:.//h3[contains(@class, 'title')]");
  });

  it("runs Test extractor and displays sample candidates and field match counts", async () => {
    const user = userEvent.setup({ delay: null });

    const sampleResult: ExtractorTestResult = {
      samples: [
        {
          evidence_url: "https://example.com/careers/1",
          external_id: "job-101",
          title: "Principal Infrastructure Engineer",
          url: "https://example.com/careers/1",
          location: "Remote - US",
          salary: "$190,000 - $220,000",
          description: "Build robust infrastructure",
          date_posted: "2026-10-15",
          employer_logo_url: "https://example.com/logo.png",
          company: "Example Tech",
        },
      ],
      field_match_counts: {
        title: 1,
        url: 1,
        location: 1,
        salary: 1,
      },
      warnings: ["Single page extraction reached max preview candidates"],
      termination: "exhausted",
    };

    const testSpy = vi.spyOn(api, "testExtractor").mockResolvedValue(sampleResult);

    render(
      <AddSourceModal
        isOpen
        initialSpec={sampleSelectorWorkflow}
        onClose={vi.fn()}
        onSourceSaved={vi.fn()}
      />
    );

    await openExtractorSettings(user);

    const testBtn = screen.getByRole("button", { name: "Test extractor" });
    await user.click(testBtn);

    await waitFor(() => {
      expect(testSpy).toHaveBeenCalled();
      expect(screen.getByText("Principal Infrastructure Engineer")).toBeTruthy();
      expect(screen.getByText(/Remote - US/)).toBeTruthy();
      expect(screen.getByText(/\$190,000 - \$220,000/)).toBeTruthy();
      expect(screen.getByText(/Single page extraction reached max preview candidates/)).toBeTruthy();
    });
  });

  it("handles Headless Browser CDP mode, runtime detection, and isolated session controls", async () => {
    const user = userEvent.setup({ delay: null });

    const runtimeStatus: BrowserRuntimeStatus = {
      available: true,
      executable_path: "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
      version: "Chrome/130.0.6723.69",
      message: null,
    };

    const sessions: BrowserSessionSummary[] = [
      {
        id: "sess-1",
        label: "Greenhouse Admin Login",
        domain: "boards.greenhouse.io",
        status: "ready",
        created_at: "2026-10-16T12:00:00Z",
        updated_at: "2026-10-16T12:00:00Z",
        last_verified_at: "2026-10-16T12:05:00Z",
      },
      {
        id: "sess-expired",
        label: "Expired SSO Session",
        domain: "sso.example.com",
        status: "needs_login",
        created_at: "2026-10-15T12:00:00Z",
        updated_at: "2026-10-15T12:00:00Z",
        last_verified_at: null,
      },
    ];

    vi.spyOn(api, "detectBrowserRuntime").mockResolvedValue(runtimeStatus);
    vi.spyOn(api, "listBrowserSessions").mockResolvedValue(sessions);
    const openSpy = vi.spyOn(api, "openBrowserSession").mockResolvedValue({
      id: "sess-new",
      label: "New Session",
      domain: "example.com",
      status: "ready",
      created_at: "2026-10-17T08:00:00Z",
      updated_at: "2026-10-17T08:00:00Z",
      last_verified_at: null,
    });
    const saveSpy = vi.spyOn(api, "saveBrowserSession").mockResolvedValue(sessions[0]);
    const clearSpy = vi.spyOn(api, "clearBrowserSession").mockResolvedValue();

    render(
      <AddSourceModal
        isOpen
        initialSpec={{
          ...sampleSelectorWorkflow,
          sources: [
            {
              ...sampleSelectorWorkflow.sources[0],
              extractor: {
                ...sampleSelectorWorkflow.sources[0].extractor,
                mode: "browser",
                session_id: "sess-1",
              },
            },
          ],
        }}
        onClose={vi.fn()}
        onSourceSaved={vi.fn()}
      />
    );

    await openExtractorSettings(user);

    // Browser runtime indicator is displayed
    await waitFor(() => {
      expect(screen.getByText(/Chrome\/130\.0\.6723\.69/i)).toBeTruthy();
    });

    // Session controls are available
    expect(screen.getByText(/Greenhouse Admin Login/i)).toBeTruthy();

    // Verify session badge transitions between Needs login and Ready
    const sessionTrigger = screen.getByRole("button", { name: /Source 1 Browser session/i });
    await user.click(sessionTrigger);
    await user.click(await screen.findByRole("option", { name: /Expired SSO Session/i }));
    expect(screen.getByText("Needs login")).toBeTruthy();

    await user.click(sessionTrigger);
    await user.click(await screen.findByRole("option", { name: /Greenhouse Admin Login/i }));
    expect(screen.getByText("Ready")).toBeTruthy();

    // Browser bounds inputs are present
    expect(screen.getByRole("spinbutton", { name: /Navigation timeout/i })).toBeTruthy();
    expect(screen.getByRole("spinbutton", { name: /Max pages/i })).toBeTruthy();

    // Open session action
    const openBtn = screen.getByRole("button", { name: "Open login session" });
    await user.click(openBtn);
    const launchBtn = screen.getByRole("button", { name: "Launch login browser" });
    await user.click(launchBtn);
    expect(openSpy).toHaveBeenCalled();

    // Save session action
    const saveBtn = screen.getByRole("button", { name: "Save session" });
    await user.click(saveBtn);
    expect(saveSpy).toHaveBeenCalledWith("sess-new");

    // Clear session action
    const clearBtn = screen.getByRole("button", { name: "Clear session" });
    await user.click(clearBtn);
    expect(clearSpy).toHaveBeenCalledWith("sess-new");
    expect(screen.getByText(/None \(unauthenticated\)/i)).toBeTruthy();
  });

  it("handles Custom Script mode, environment checks, truthful disclosure, and explicit trust gate", async () => {
    const user = userEvent.setup({ delay: null });

    const envStatus: ScriptEnvironmentStatus = {
      available: true,
      runtime: "python",
      executable_path: "/usr/bin/python3",
      version: "Python 3.12.3",
      missing_modules: [],
      message: null,
    };

    const checkEnvSpy = vi.spyOn(api, "checkScriptEnvironment").mockResolvedValue(envStatus);
    const trustSpy = vi.spyOn(api, "trustWorkflowScript").mockResolvedValue({
      ...sampleScriptSource,
      needs_review: false,
      script_trusted: true,
    });

    render(
      <AddSourceModal
        isOpen
        initialSource={sampleScriptSource}
        onClose={vi.fn()}
        onSourceSaved={vi.fn()}
      />
    );

    await openExtractorSettings(user);

    // Truthful privilege disclosure is visible
    expect(
      screen.getByText(/Custom scripts execute with your local user account privileges/i)
    ).toBeTruthy();

    // Trust banner is visible because script is untrusted / needs review
    expect(
      screen.getByText(/This script configuration requires review and explicit trust/i)
    ).toBeTruthy();

    // Check environment button
    const checkBtn = screen.getByRole("button", { name: "Check environment" });
    await user.click(checkBtn);
    await waitFor(() => {
      expect(checkEnvSpy).toHaveBeenCalled();
      expect(screen.getByText(/Python 3\.12\.3/i)).toBeTruthy();
    });

    // Trust saved script button
    const trustBtn = screen.getByRole("button", { name: "Trust saved script" });
    await user.click(trustBtn);
    await waitFor(() => {
      expect(trustSpy).toHaveBeenCalledWith(
        "src-script-1",
        "abc123sha256fingerprint",
        true
      );
    });

    expect(screen.getByText("Saved script trusted. Enable the workflow explicitly when ready.")).toBeTruthy();
    expect(
      screen.queryByText(/This script configuration requires review and explicit trust/i)
    ).toBeNull();
  });

  it("disables trust button and displays save-first message when draft script is modified", async () => {
    const user = userEvent.setup({ delay: null });

    const trustSpy = vi.spyOn(api, "trustWorkflowScript");

    render(
      <AddSourceModal
        isOpen
        initialSource={sampleScriptSource}
        onClose={vi.fn()}
        onSourceSaved={vi.fn()}
      />
    );

    await openExtractorSettings(user);

    const pathInput = screen.getByRole("textbox", { name: /Script file path/i });
    await user.clear(pathInput);
    await user.type(pathInput, "./scripts/modified_scraper.py");

    // The trust button should be disabled / hidden with save-first message
    expect(
      screen.getByText(/Save the workflow to fingerprint your changes, then trust it/i)
    ).toBeTruthy();
    const trustBtn = screen.getByRole("button", { name: "Trust saved script" });
    expect((trustBtn as HTMLButtonElement).disabled).toBe(true);
    expect(trustSpy).not.toHaveBeenCalled();
  });
});

it("saves an executable draft disabled for review without previewing or running it", async () => {
  const user = userEvent.setup({ delay: null });
  const validate = vi.spyOn(api,"validateWorkflow").mockImplementation(async spec => spec as WorkflowSpec);
  const save = vi.spyOn(api,"saveWorkflow").mockResolvedValue({...sampleScriptSource,enabled:false});
  const preview = vi.spyOn(api,"previewWorkflow");
  const run = vi.spyOn(api,"runSourceNow");
  render(<AddSourceModal isOpen initialSpec={sampleScriptWorkflow} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  await user.click(screen.getByRole("button",{name:"Save disabled for review"}));
  await screen.findByText(/Saved disabled\. Reopen this workflow/);
  expect(validate).toHaveBeenCalled();
  expect(save.mock.calls[0][0].metadata.enabled).toBe(false);
  expect(save.mock.calls[0][0].sources.every(s=>!s.enabled)).toBe(true);
  expect(preview).not.toHaveBeenCalled();expect(run).not.toHaveBeenCalled();
  expect((screen.getByRole("button",{name:"Run now"}) as HTMLButtonElement).disabled).toBe(true);
});

it("reviews a multi-source child's stored fingerprint instead of the parent's fingerprint", async () => {
  const user=userEvent.setup({delay:null});
  const parentId="parent-static";
  const parentSpec:WorkflowSpec={...sampleScriptWorkflow,sources:[{...sampleSelectorWorkflow.sources[0],id:parentId},...sampleScriptWorkflow.sources]};
  const parent:Source={...sampleScriptSource,id:parentId,script_sha256:null,config:{__workflow_spec:parentSpec}};
  const child:Source={...sampleScriptSource,enabled:false};
  const read=vi.spyOn(api,"getSource").mockResolvedValue(child);
  const trust=vi.spyOn(api,"trustWorkflowScript").mockResolvedValue({...child,script_trusted:true,needs_review:false});
  render(<AddSourceModal isOpen initialSource={parent} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  await openExtractorSettings(user);
  await user.click(await screen.findByRole("button",{name:"Trust saved script"}));
  expect(read).toHaveBeenCalledWith(child.id);
  expect(trust).toHaveBeenCalledWith(child.id,child.script_sha256,true);
  expect(trust.mock.calls[0][0]).not.toBe(parentId);
  expect(await screen.findByText(/Saved script trusted\. Enable the workflow explicitly/)).toBeTruthy();
});

it("requires saving when the owned child's script differs from the parent snapshot being displayed", async () => {
  const user=userEvent.setup({delay:null});
  const parentId="parent-static";
  const parentSpec:WorkflowSpec={...sampleScriptWorkflow,sources:[{...sampleSelectorWorkflow.sources[0],id:parentId},...sampleScriptWorkflow.sources]};
  const parent:Source={...sampleScriptSource,id:parentId,script_sha256:null,config:{__workflow_spec:parentSpec}};
  vi.spyOn(api,"getSource").mockResolvedValue({...sampleScriptSource,config:{script:{runtime:"js",code:"different saved executable"}}});
  const trust=vi.spyOn(api,"trustWorkflowScript");
  render(<AddSourceModal isOpen initialSource={parent} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  await openExtractorSettings(user);
  const button=await screen.findByRole("button",{name:"Trust saved script"});
  expect((button as HTMLButtonElement).disabled).toBe(true);
  await user.click(button);expect(trust).not.toHaveBeenCalled();
});
