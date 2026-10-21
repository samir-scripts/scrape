interface KeyHintProps {
  keys: string;
  label?: string;
  className?: string;
}

export function KeyHint({ keys, label, className = "" }: KeyHintProps) {
  return (
    <span
      className={`inline-flex items-center gap-1.5 font-mono text-xs uppercase ${className}`}
    >
      <span className="border border-current px-1 py-0.5 tracking-tighter leading-none select-none">
        {keys}
      </span>
      {label && <span>{label}</span>}
    </span>
  );
}
