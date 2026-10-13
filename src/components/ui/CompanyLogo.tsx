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
  const [imgError, setImgError] = useState(false);

  const sizeClasses = {
    sm: "w-[18px] h-[18px] text-[9px]",
    md: "w-[24px] h-[24px] text-[11px]",
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

  // Derive favicon fallback url if iconUrl wasn't directly provided
  const derivedUrl =
    iconUrl ||
    (() => {
      // If company has a domain suffix or recognizable name, build google favicon url
      const slug = company.toLowerCase().replace(/[^a-z0-9]/g, "");
      if (slug.includes("cloudflare")) return "https://www.google.com/s2/favicons?domain=cloudflare.com&sz=64";
      if (slug.includes("stripe")) return "https://www.google.com/s2/favicons?domain=stripe.com&sz=64";
      if (slug.includes("figma")) return "https://www.google.com/s2/favicons?domain=figma.com&sz=64";
      if (slug.includes("google")) return "https://www.google.com/s2/favicons?domain=google.com&sz=64";
      if (slug.includes("meta")) return "https://www.google.com/s2/favicons?domain=meta.com&sz=64";
      if (slug.includes("amazon")) return "https://www.google.com/s2/favicons?domain=amazon.com&sz=64";
      if (slug.includes("apple")) return "https://www.google.com/s2/favicons?domain=apple.com&sz=64";
      if (company.includes(".")) {
        return `https://www.google.com/s2/favicons?domain=${encodeURIComponent(company)}&sz=64`;
      }
      return null;
    })();

  if (derivedUrl && !imgError) {
    return (
      <div
        className={`inline-flex items-center justify-center border border-[var(--color-ink)] bg-[var(--color-paper)] shrink-0 overflow-hidden ${sizeClasses} ${className}`}
        title={company}
      >
        <img
          src={derivedUrl}
          alt={company}
          onError={() => setImgError(true)}
          className="w-full h-full object-contain p-[2px]"
          style={{ imageRendering: "pixelated" }}
        />
      </div>
    );
  }

  return (
    <div
      className={`inline-flex items-center justify-center border border-[var(--color-ink)] bg-[var(--color-paper-2)] text-[var(--color-ink)] font-mono font-bold tracking-tighter shrink-0 select-none ${sizeClasses} ${className}`}
      title={company}
      aria-label={`${company} logo`}
    >
      {getInitials(company)}
    </div>
  );
}
