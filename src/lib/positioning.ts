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
  const width = Math.min(panelWidth, Math.max(320, windowWidth - 24));

  if (!triggerRect) {
    return {
      top: 48,
      left: 12,
      maxHeight: Math.max(300, windowHeight - 48 - 16),
      width,
    };
  }

  const top = triggerRect.bottom + gap;
  // Left-align with button, clamped within viewport safe margins (12px padding)
  const maxLeft = Math.max(12, windowWidth - width - 12);
  const left = Math.max(12, Math.min(triggerRect.left, maxLeft));
  const maxHeight = Math.max(220, windowHeight - top - 16);

  return {
    top,
    left,
    maxHeight,
    width,
  };
}
