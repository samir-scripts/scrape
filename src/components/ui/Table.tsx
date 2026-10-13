import type {
  TableHTMLAttributes,
  HTMLAttributes,
  ThHTMLAttributes,
  TdHTMLAttributes,
  ReactNode,
} from "react";

interface TableProps extends TableHTMLAttributes<HTMLTableElement> {
  children: ReactNode;
}

export function Table({ className = "", children, ...props }: TableProps) {
  return (
    <div className="w-full border border-[var(--color-ink)] overflow-x-auto bg-[var(--color-paper)]">
      <table
        className={`w-full text-left text-xs font-mono border-collapse ${className}`}
        {...props}
      >
        {children}
      </table>
    </div>
  );
}

export function TableHead({
  className = "",
  children,
  ...props
}: HTMLAttributes<HTMLTableSectionElement>) {
  return (
    <thead
      className={`border-b border-[var(--color-ink)] bg-[var(--color-paper-2)] text-[var(--color-ink)] uppercase text-xs select-none ${className}`}
      {...props}
    >
      {children}
    </thead>
  );
}

export function TableHeader({
  className = "",
  children,
  ...props
}: ThHTMLAttributes<HTMLTableCellElement>) {
  return (
    <th
      className={`px-3 py-2 font-bold tracking-wider border-r border-[var(--color-ink)] last:border-r-0 ${className}`}
      {...props}
    >
      {children}
    </th>
  );
}

export function TableBody({
  className = "",
  children,
  ...props
}: HTMLAttributes<HTMLTableSectionElement>) {
  return <tbody className={`divide-y divide-[var(--color-ink)] ${className}`} {...props}>{children}</tbody>;
}

export function TableRow({
  className = "",
  children,
  ...props
}: HTMLAttributes<HTMLTableRowElement>) {
  return (
    <tr
      className={`hover:bg-[var(--color-paper-2)] transition-colors ${className}`}
      {...props}
    >
      {children}
    </tr>
  );
}

export function TableCell({
  className = "",
  children,
  ...props
}: TdHTMLAttributes<HTMLTableCellElement>) {
  return (
    <td
      className={`px-3 py-2 border-r border-[var(--color-ink)] last:border-r-0 ${className}`}
      {...props}
    >
      {children}
    </td>
  );
}
