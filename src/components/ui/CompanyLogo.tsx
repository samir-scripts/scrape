import { useState } from "react";

export interface CompanyLogoProps {
  company: string;
  iconUrl?: string | null;
  size?: "sm" | "md" | "lg";
  className?: string;
}

export function CompanyLogo({
  company,
  iconUrl,
  size = "md",
  className = "",
}: CompanyLogoProps) {
  const [failedUrl, setFailedUrl] = useState<string | null>(null);

  const sizeClasses = {
    sm: "w-[18px] h-[18px] text-[9px]",
    md: "w-[24px] h-[24px] text-xs",
    lg: "w-[32px] h-[32px] text-[13px]",
  }[size];

  // Helper to extract 1-2 initials
  const getInitials = (name: string): string => {
    if (!name) return "?";
    const clean = name.replace(/[^a-zA-Z0-9\s]/g, "").trim();
    const parts = clean.split(/\s+/);
    if (parts.length >= 2 && parts[0] && parts[1]) {
      return (parts[0][0] + parts[1][0]).toUpperCase();
    }
    return (clean.slice(0, 2) || "?").toUpperCase();
  };

  // Only source-provided metadata identifies an actual company logo.
  const derivedUrl = iconUrl;

  if (derivedUrl && failedUrl !== derivedUrl) {
    return (
      <div
        className={`inline-flex items-center justify-center border border-[var(--color-ink)] bg-[var(--palette-cream)] shrink-0 overflow-hidden ${sizeClasses} ${className}`}
        title={company}
      >
        <img
          src={derivedUrl}
          alt={company}
          onError={() => setFailedUrl(derivedUrl)}
          className="w-full h-full object-contain p-[2px]"
          style={{ imageRendering: "auto" }}
        />
      </div>
    );
  }

  return (
    <div
      className={`inline-flex items-center justify-center border border-[var(--color-ink)] bg-[var(--color-paper-2)] text-[var(--color-ink)] font-mono font-bold tracking-tighter shrink-0 select-none ${sizeClasses} ${className}`}
      title={company}
      aria-label={`${company} initials`}
    >
      {getInitials(company)}
    </div>
  );
}
