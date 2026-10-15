import { INGESTION_LIMITS } from "../../../bindings/ingestionLimits";
import { useEffect, useId, useState } from "react";
import { Input } from "../../ui/Input";

export function JobLimitInput({ value, onChange }: { value: number; onChange: (value: number | null) => void }) {
  const [text, setText] = useState(String(value));
  const id = useId();
  useEffect(() => setText(String(value)), [value]);
  const parse = (value: string) => /^\d+$/.test(value) && Number(value) >= 1 && Number(value) <= INGESTION_LIMITS.maximum ? Number(value) : null;
  const valid = parse(text) != null;
  return <div className="grid gap-2 text-sm">
    <label htmlFor={id}>Maximum matching jobs per run</label>
    <Input id={id} inputMode="numeric" value={text} aria-invalid={!valid} aria-describedby={`${id}-help`}
      onChange={event => {
        const next = event.target.value;
        setText(next);
        onChange(parse(next));
      }} />
    <p id={`${id}-help`} className={!valid ? "text-[var(--color-danger)]" : "text-[var(--color-ink-mute)]"}>
      {!valid ? `Enter a whole number from 1 to ${INGESTION_LIMITS.maximum.toLocaleString("en-US")}. ` : ""}
      Stop after this many matching, unblocked postings. Existing postings refreshed by this run count. Preview accepts at most {INGESTION_LIMITS.preview}.
    </p>
  </div>;
}
