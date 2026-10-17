// @vitest-environment jsdom
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";
import { api } from "../lib/api";

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

describe("Sources Editor (Section 5)", () => {
  it("supports adding, reordering, and removing sources with at-least-one-source protection", async () => {
    const user = userEvent.setup({ delay: null });
    render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);

    expect(screen.getByText("Sources (1)")).toBeTruthy();
    const removeBtn = screen.getByRole("button", { name: /Remove source/ });
    expect(removeBtn.getAttribute("disabled")).not.toBeNull();

    // Add a second source
    await user.click(screen.getByRole("button", { name: "Add source" }));
    expect(screen.getByText("Sources (2)")).toBeTruthy();

    // Fill source 1 details
    const source1Url = screen.getByRole("textbox", { name: "Source URL or board identifier" });
    await user.type(source1Url, "https://boards.greenhouse.io/cloudflare");
    const source1Label = screen.getByRole("textbox", { name: "Source label" });
    await user.clear(source1Label);
    await user.type(source1Label, "Cloudflare Source");

    // Fill source 2 details
    const source2Url = screen.getByRole("textbox", { name: "Source 2 URL or board identifier" });
    await user.type(source2Url, "https://jobs.lever.co/figma");
    const source2Label = screen.getByRole("textbox", { name: "Source 2 label" });
    await user.clear(source2Label);
    await user.type(source2Label, "Figma Source");

    // Reorder: Move source 1 down
    const moveDownBtn = screen.getByRole("button", { name: "Move source Cloudflare Source down" });
    expect(moveDownBtn.getAttribute("disabled")).toBeNull();
    await user.click(moveDownBtn);

    // After reordering, the first source is now Figma Source
    expect((screen.getByRole("textbox", { name: "Source label" }) as HTMLInputElement).value).toBe("Figma Source");
    expect((screen.getByRole("textbox", { name: "Source 2 label" }) as HTMLInputElement).value).toBe("Cloudflare Source");

    // Remove source 2 (which is now Cloudflare Source)
    const removeSource2 = screen.getByRole("button", { name: "Remove source Cloudflare Source" });
    await user.click(removeSource2);

    expect(screen.getByText("Sources (1)")).toBeTruthy();
    expect((screen.getByRole("textbox", { name: "Source label" }) as HTMLInputElement).value).toBe("Figma Source");
    // After removal, the remaining source cannot be removed
    expect(screen.getByRole("button", { name: /Remove source/ }).getAttribute("disabled")).not.toBeNull();
  });

  it("detects and rejects duplicate normalized endpoints across sources", async () => {
    const user = userEvent.setup({ delay: null });
    render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);

    // Source 1: stripe full greenhouse URL
    const source1Url = screen.getByRole("textbox", { name: "Source URL or board identifier" });
    await user.type(source1Url, "https://boards.greenhouse.io/stripe");

    // Add source 2
    await user.click(screen.getByRole("button", { name: "Add source" }));

    // Source 2: enter same endpoint with trailing slash
    const source2Url = screen.getByRole("textbox", { name: "Source 2 URL or board identifier" });
    await user.type(source2Url, "https://boards.greenhouse.io/stripe/");

    // Duplicate alert is shown
    expect(screen.getByText(/Duplicate source endpoint detected/)).toBeTruthy();

    // Trying to continue to Matching fails with error
    await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
    const alerts = screen.getAllByRole("alert");
    expect(alerts.some(a => /Duplicate source endpoint/i.test(a.textContent || ""))).toBe(true);
  });

  it("triggers independent source detection per source card", async () => {
    const user = userEvent.setup({ delay: null });
    const detectSpy = vi.spyOn(api, "detectSource").mockResolvedValue({
      kind: "greenhouse",
      identifier: "stripe",
      name: "Stripe",
    });

    render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);
    const source1Url = screen.getByRole("textbox", { name: "Source URL or board identifier" });
    await user.type(source1Url, "https://boards.greenhouse.io/stripe");

    const detectBtn = screen.getByRole("button", { name: "Detect source 1" });
    await user.click(detectBtn);

    await waitFor(() => {
      expect(detectSpy).toHaveBeenCalledWith("https://boards.greenhouse.io/stripe");
      expect((screen.getByRole("textbox", { name: "Workflow name" }) as HTMLInputElement).value).toBe("Stripe");
      expect((screen.getByRole("textbox", { name: "Source label" }) as HTMLInputElement).value).toBe("Stripe");
    });
  });

  it("warns when all sources in the workflow are disabled", async () => {
    const user = userEvent.setup();
    render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);

    const enableCheckbox = screen.getByRole("checkbox", { name: /Enable/ });
    expect((enableCheckbox as HTMLInputElement).checked).toBe(true);

    await user.click(enableCheckbox);
    expect((enableCheckbox as HTMLInputElement).checked).toBe(false);

    expect(screen.getByText(/At least one source must be enabled/)).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
    const alerts = screen.getAllByRole("alert");
    expect(alerts.some(a => /At least one source must be enabled/i.test(a.textContent || ""))).toBe(true);
  });
});
