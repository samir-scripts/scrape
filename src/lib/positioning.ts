/**
 * Viewport positioning & geometric anchoring engine.
 * Pure math calculations for anchored popovers, tooltips, and dropdown panels.
 */

export type TooltipPosition = "top" | "bottom";

export interface TooltipCoordinates {
  top: number;
  left: number;
  placement: TooltipPosition;
}

export interface BoundingBox {
  top: number;
  bottom: number;
  left: number;
  right: number;
  width: number;
  height: number;
}

export interface FilterDropdownCoordinates {
  top: number;
  left: number;
  maxHeight: number;
  width: number;
}

/**
 * Calculates theoretical bounding coordinates for tooltips.
 */
export function calculateTooltipCoordinates(
  rect: { top: number; bottom: number; left: number; width: number },
  position: TooltipPosition = "top",
  tooltipWidth: number = 0,
  windowWidth: number = typeof window !== "undefined" ? window.innerWidth : 1024,
  tooltipHeight: number = 24,
  windowHeight: number = typeof window !== "undefined" ? window.innerHeight : 768,
  gap: number = 4
): TooltipCoordinates {
  const height = tooltipHeight > 0 ? tooltipHeight : 24;
  let resolvedPlacement: TooltipPosition = position;
  let top = 0;

  if (position === "top") {
    top = rect.top - (tooltipHeight > 0 ? height + gap : 28);
  } else {
    top = rect.bottom + gap;
  }

  // Vertical safety margins: flip if overflowing screen boundaries
  if (position === "top") {
    if (top < 4) {
      top = rect.bottom + gap;
      resolvedPlacement = "bottom";
    }
  } else {
    if (top + height > windowHeight - 4) {
      top = Math.max(4, rect.top - height - gap);
      resolvedPlacement = "top";
    }
  }

  // Center horizontally relative to trigger
  let left = rect.left + rect.width / 2;
  const padding = 12; // safe viewport margin

  if (tooltipWidth > 0) {
    const halfWidth = tooltipWidth / 2;
    if (tooltipWidth >= windowWidth - padding * 2) {
      left = windowWidth / 2;
    } else {
      if (left - halfWidth < padding) {
        left = padding + halfWidth;
      } else if (left + halfWidth > windowWidth - padding) {
        left = windowWidth - padding - halfWidth;
      }
    }
  } else {
    // Backward compatibility when tooltipWidth is not yet measured (0)
    if (left < 40) left = 40;
  }

  return { top, left, placement: resolvedPlacement };
}

/**
 * Calculates the exact rendered bounding box of a tooltip anchored with CSS transform translate.
 * Guarantees zero overlap with trigger element.
 */
export function getTooltipVisualBounds(
  triggerRect: { top: number; bottom: number; left: number; width: number },
  placement: TooltipPosition,
  tooltipWidth: number,
  tooltipHeight: number,
  offset: number = 6,
  windowWidth: number = 1024
): BoundingBox {
  const height = tooltipHeight > 0 ? tooltipHeight : 24;
  const width = tooltipWidth > 0 ? tooltipWidth : 120;
  const padding = 12;

  let top = 0;
  let bottom = 0;

  if (placement === "top") {
    bottom = triggerRect.top - offset;
    top = bottom - height;
  } else {
    top = triggerRect.bottom + offset;
    bottom = top + height;
  }

  let center = triggerRect.left + triggerRect.width / 2;
  const halfWidth = width / 2;

  if (width >= windowWidth - padding * 2) {
    center = windowWidth / 2;
  } else if (center - halfWidth < padding) {
    center = padding + halfWidth;
  } else if (center + halfWidth > windowWidth - padding) {
    center = windowWidth - padding - halfWidth;
  }

  const left = center - halfWidth;
  const right = center + halfWidth;

  return { top, bottom, left, right, width, height };
}

export interface PopoverAnchorOptions {
  panelWidth?: number;
  align?: "left" | "right";
  gap?: number;
  viewportPadding?: number;
  minWidth?: number;
}

/**
 * Calculates anchored popover coordinates tethered directly beneath any trigger button.
 * Supports "left" or "right" alignment with viewport boundary clamping.
 */
export function calculateAnchoredPopoverCoordinates(
  triggerRect: { top: number; bottom: number; left: number; right: number; width: number } | null,
  windowWidth: number = typeof window !== "undefined" ? window.innerWidth : 1024,
  windowHeight: number = typeof window !== "undefined" ? window.innerHeight : 768,
  options: PopoverAnchorOptions = {}
): FilterDropdownCoordinates {
  const panelWidth = options.panelWidth ?? 400;
  const align = options.align ?? "left";
  const gap = options.gap ?? 4;
  const padding = options.viewportPadding ?? 12;
  const minWidth = options.minWidth ?? 300;

  const width = Math.min(panelWidth, Math.max(minWidth, windowWidth - padding * 2));

  if (!triggerRect) {
    return {
      top: 48,
      left: align === "right" ? Math.max(padding, windowWidth - width - padding) : padding,
      maxHeight: Math.max(220, windowHeight - 48 - 16),
      width,
    };
  }

  const top = triggerRect.bottom + gap;
  let left = 0;
  if (align === "right") {
    left = triggerRect.right - width;
  } else {
    left = triggerRect.left;
  }

  // Viewport clamping
  const maxLeft = Math.max(padding, windowWidth - width - padding);
  left = Math.max(padding, Math.min(left, maxLeft));
  const maxHeight = Math.max(220, windowHeight - top - 16);

  return {
    top,
    left,
    maxHeight,
    width,
  };
}

/**
 * Calculates anchored popover coordinates tethered directly beneath the FILTERS trigger button.
 * Clamps coordinates within viewport boundaries to prevent screen overflow.
 */
export function calculateFilterDropdownCoordinates(
  triggerRect: { top: number; bottom: number; left: number; right: number; width: number } | null,
  windowWidth: number = typeof window !== "undefined" ? window.innerWidth : 1024,
  windowHeight: number = typeof window !== "undefined" ? window.innerHeight : 768,
  panelWidth: number = 420,
  gap: number = 4
): FilterDropdownCoordinates {
  return calculateAnchoredPopoverCoordinates(triggerRect, windowWidth, windowHeight, {
    panelWidth,
    align: "left",
    gap,
    minWidth: 320,
  });
}
