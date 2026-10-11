import { describe, it, expect, vi } from "vitest";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Toggle } from "../components/ui/Toggle";

describe("Mechanical Toggle Switch Component", () => {
  it("renders with role='switch' and aria-checked='true' when checked", () => {
    const html = renderToStaticMarkup(
      React.createElement(Toggle, {
        checked: true,
        onChange: () => {},
        label: "AUTOMATIC UPDATES",
      })
    );

    expect(html).toContain('role="switch"');
    expect(html).toContain('aria-checked="true"');
    expect(html).toContain('aria-disabled="false"');
    expect(html).toContain("AUTOMATIC UPDATES");
    // Verify brutalist mechanical styling classes
    expect(html).toContain("w-8 h-4");
    expect(html).toContain("translate-x-3.5");
  });

  it("renders with aria-checked='false' and 0 offset thumb when unchecked", () => {
    const html = renderToStaticMarkup(
      React.createElement(Toggle, {
        checked: false,
        onChange: () => {},
        label: "ENABLE SCRAPER",
      })
    );

    expect(html).toContain('role="switch"');
    expect(html).toContain('aria-checked="false"');
    expect(html).toContain("ENABLE SCRAPER");
    expect(html).toContain("translate-x-0");
  });

  it("renders small size classes when size='sm'", () => {
    const html = renderToStaticMarkup(
      React.createElement(Toggle, {
        checked: true,
        onChange: () => {},
        size: "sm",
      })
    );

    expect(html).toContain("w-7 h-3.5");
    expect(html).toContain("translate-x-3");
  });

  it("sets aria-disabled and disabled styles when disabled", () => {
    const html = renderToStaticMarkup(
      React.createElement(Toggle, {
        checked: false,
        onChange: () => {},
        disabled: true,
      })
    );

    expect(html).toContain('aria-disabled="true"');
    expect(html).toContain('tabindex="-1"');
    expect(html).toContain("cursor-not-allowed");
    expect(html).toContain("opacity-50");
  });

  it("handles keyboard events (Enter and Space) on switch container element", () => {
    const onChange = vi.fn();
    const element = Toggle({
      checked: false,
      onChange,
      label: "KEYBOARD TEST",
    });

    expect(element.props.role).toBe("switch");
    expect(element.props["aria-checked"]).toBe(false);

    // Simulate click
    element.props.onClick();
    expect(onChange).toHaveBeenCalledWith(true);

    // Simulate Space press
    const spaceEvent = { key: " ", preventDefault: vi.fn() };
    element.props.onKeyDown(spaceEvent);
    expect(spaceEvent.preventDefault).toHaveBeenCalled();
    expect(onChange).toHaveBeenCalledWith(true);

    // Simulate Enter press
    const enterEvent = { key: "Enter", preventDefault: vi.fn() };
    element.props.onKeyDown(enterEvent);
    expect(enterEvent.preventDefault).toHaveBeenCalled();
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("ignores click and keyboard events when disabled", () => {
    const onChange = vi.fn();
    const element = Toggle({
      checked: true,
      onChange,
      disabled: true,
    });

    element.props.onClick();
    expect(onChange).not.toHaveBeenCalled();

    const enterEvent = { key: "Enter", preventDefault: vi.fn() };
    element.props.onKeyDown(enterEvent);
    expect(onChange).not.toHaveBeenCalled();
  });
});
