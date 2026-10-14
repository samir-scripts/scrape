// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { useState } from "react";
import { FavoriteButton } from "../components/ui/FavoriteButton";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Icon } from "../components/ui/Icon";
import { IconButton } from "../components/ui/IconButton";

afterEach(cleanup);

describe("shared SVG controls", () => {
  it("has one accessible name, responds to keyboard and skips disabled actions", async () => {
    const open = vi.fn(), disabled = vi.fn();
    render(<><IconButton label="Open Engineer at Acme" onClick={open}><Icon name="external" /></IconButton>
      <IconButton label="Upload workflow" disabled onClick={disabled}><Icon name="upload" /></IconButton>
      <button>Next control</button></>);
    const user = userEvent.setup();
    await user.tab();
    const action = screen.getByRole("button", { name: "Open Engineer at Acme" });
    expect(document.activeElement).toBe(action);
    expect(action.querySelector("svg")?.getAttribute("aria-hidden")).toBe("true");
    expect(screen.queryByRole("img")).toBeNull();
    await user.keyboard("{Enter}");
    expect(open).toHaveBeenCalledTimes(1);
    await user.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Next control" }));
    await user.click(screen.getByRole("button", { name: "Upload workflow" }));
    expect(disabled).not.toHaveBeenCalled();
  });
  it("toggles the shared star with keyboard while retaining its accessible pressed state", async () => {
    function Harness() {
      const [favorited, setFavorited] = useState(false);
      return <FavoriteButton favorited={favorited} onToggle={() => setFavorited(value => !value)} />;
    }
    render(<Harness />);
    const user = userEvent.setup();
    await user.tab(); await user.keyboard("{Enter}");
    expect(screen.getByRole("button", { name: "Unfavorite posting" }).getAttribute("aria-pressed")).toBe("true");
    await user.keyboard(" ");
    expect(screen.getByRole("button", { name: "Favorite posting" }).getAttribute("aria-pressed")).toBe("false");
  });

});
