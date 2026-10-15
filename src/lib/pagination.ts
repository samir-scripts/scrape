// The UI uses one-based pages; repository offsets cross this boundary only here.
export function pagination(total: number | undefined, page: number, pageSize: number) {
  if (!Number.isSafeInteger(pageSize) || pageSize < 1) throw new Error("Invalid page size");
  const pageCount = total === undefined ? undefined : Math.ceil(Math.max(0, total) / pageSize);
  const current = Math.max(1, Math.min(page, Math.max(1, pageCount ?? page)));
  const offset = (current - 1) * pageSize;
  return { page: current, pageCount, offset, start: total ? offset + 1 : 0, end: total === undefined ? 0 : Math.min(offset + pageSize, total) };
}
