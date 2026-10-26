import { INGESTION_LIMITS } from "../../../bindings/ingestionLimits";
import { useEffect, useId, useState } from "react";
import { Input } from "../../ui/Input";

export function JobLimitInput({
  value,
  onChange,
  label = "Target new Discovered jobs per run",
}: {
  value: number;
  onChange: (value: number | null) => void;
  label?: string;
}) {
  const [text, setText] = useState(String(value));
  const id = useId();
  useEffect(() => setText(String(value)), [value]);
  const parse = (value: string) => /^\d+$/.test(value) && Number(value) >= 1 && Number(value) <= INGESTION_LIMITS.maximum ? Number(value) : null;
  const valid = parse(text) != null;
  return <div className="grid gap-2 text-sm">
    <label htmlFor={id}>{label}</label>
    <Input id={id} inputMode="numeric" value={text} aria-invalid={!valid} aria-describedby={`${id}-help`}
      onChange={event => {
        const next = event.target.value;
        setText(next);
        onChange(parse(next));
      }} />
    <p id={`${id}-help`} className={!valid ? "text-[var(--color-danger)]" : "text-[var(--color-ink-mute)]"}>
      {!valid ? `Enter a whole number from 1 to ${INGESTION_LIMITS.maximum.toLocaleString("en-US")}. ` : ""}
      Refreshes do not count. Safety bounds or source exhaustion can leave the target unmet. Preview does not save jobs and samples at most {Math.min(value, INGESTION_LIMITS.preview)} eligible matches.
    </p>
  </div>;
}
