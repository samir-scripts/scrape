import type { SVGProps } from "react";

// Original rectilinear paths on one 24-unit grid; no font or external asset dependency.
const paths = {
  upload: "M4 16v5h16v-5M12 17V3M6 9h3V6h3M18 9h-3V6h-3",
  download: "M4 16v5h16v-5M12 3v14M6 11h3v3h3M18 11h-3v3h-3",
  left: "M20 12H4M10 6H7v3H4v6h3v3h3",
  right: "M4 12h16M14 6h3v3h3v6h-3v3h-3",
  up: "M12 20V4M6 10V7h3V4h6v3h3v3",
  down: "M12 4v16M6 14v3h3v3h6v-3h3v-3",
  refresh: "M20 10V4h-6M20 4H7v3H4v10h3v3h10v-3h3v-3",
  close: "m6 6 12 12M18 6 6 18",
  trash: "M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7",
  star: "M12 2 15 8 22 9 17 14 18 21 12 18 6 21 7 14 2 9 9 8Z",
  external: "M13 3h8v8M21 3 11 13M9 4H3v17h17v-6",
  drag: "M7 4h2v2H7zM15 4h2v2h-2zM7 11h2v2H7zM15 11h2v2h-2zM7 18h2v2H7zM15 18h2v2h-2z",
  board: "M3 3h18v18H3zM9 3v18M15 3v18",
  list: "M3 5h3v3H3zM10 6h11M3 11h3v3H3zM10 12h11M3 17h3v3H3zM10 18h11",
  search: "M3 3h12v12H3zM15 15l6 6",
  filter: "M3 4h18v4h-3v3h-3v10l-6-3v-7H6V8H3z",
  bell: "M10 2h4v3h4v11h3v3H3v-3h3V5h4zM10 22h4",
  terminal: "M2 4h20v16H2zM6 8l4 4-4 4M13 16h5",
} as const;
export type IconName = keyof typeof paths;
export const ICON_SIZES = { sm: 14, md: 16, lg: 20 } as const;

export function Icon({ name, size = "md", filled = false, ...props }: Omit<SVGProps<SVGSVGElement>, "name" | "children"> & {
  name: IconName;
  size?: keyof typeof ICON_SIZES;
  filled?: boolean;
}) {
  return <svg {...props} aria-hidden="true" focusable="false" width={ICON_SIZES[size]} height={ICON_SIZES[size]}
    viewBox="0 0 24 24" fill={filled ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.8"
    strokeLinecap="square" strokeLinejoin="miter" className={`ui-icon ${props.className ?? ""}`}>
    <path d={paths[name]} />
  </svg>;
}
