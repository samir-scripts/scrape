import { loadCompanyLogoAsset } from "../../lib/companyLogoCache";
import { useEffect, useState } from "react";
import type { CompanyLogo as LogoReport } from "../../bindings/CompanyLogo";

export interface CompanyLogoProps {
  company: string;
  iconUrl?: string | null;
  logo?: LogoReport | null;
  size?: "sm" | "md" | "lg";
  className?: string;
}

export function CompanyLogo({
  company,
  iconUrl,
  logo,
  size = "md",
  className = "",
}: CompanyLogoProps) {
  const [failedUrl, setFailedUrl] = useState<string | null>(null);
  const [cacheFailure, setCacheFailure] = useState<string | null>(null);
  const [cached, setCached] = useState<{reference: string; url: string} | null>(null);
  useEffect(() => {
    let active=true;
    const reference=logo?.cached_asset_reference;
    if (reference) void loadCompanyLogoAsset(reference).then(url=>{if(active){setCached({reference,url});setCacheFailure(null);}}).catch(()=>{if(active){setCached(null);setCacheFailure(reference);}});
    return ()=>{active=false;};
  },[logo?.cached_asset_reference,logo?.checked_at]);

  const sizeClasses = {
    sm: "w-[24px] h-[24px] text-xs",
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
  const derivedUrl = logo?.asset || (cached?.reference === logo?.cached_asset_reference ? cached?.url : null) || (logo ? null : iconUrl);
  const attemptKey = `${company}:${derivedUrl}:${logo?.checked_at || "legacy"}`;
  const cacheUnavailable = !!logo?.cached_asset_reference && cacheFailure === logo.cached_asset_reference;
  const diagnostic = (cacheUnavailable ? "Cached employer logo could not be loaded; retry after refreshing." : logo?.diagnostic) || (logo?.status === "missing_metadata" ? "No employer logo metadata" : logo?.status === "download_failed" ? "Employer logo download failed" : logo?.status === "decode_failed" ? "Employer logo decoding failed" : "");

  if (derivedUrl && failedUrl !== attemptKey) {
    return (
      <span
        className={`inline-flex items-center justify-center border border-[var(--color-ink)] bg-[var(--color-logo-surface)] shrink-0 overflow-hidden ${sizeClasses} ${className}`}
        title={diagnostic ? `${company}: ${diagnostic}` : company}
      >
        <img
          src={derivedUrl}
          data-logo-status={logo?.status || "unverified"}
          referrerPolicy="no-referrer"
          alt={company}
          onError={() => setFailedUrl(attemptKey)}
          className="w-full h-full object-contain"
          style={{ imageRendering: "auto" }}
        />
      </span>
    );
  }

  return (
    <span
      className={`inline-flex items-center justify-center border border-[var(--color-ink)] bg-[var(--color-paper-2)] text-[var(--color-ink)] font-mono font-bold tracking-tighter shrink-0 select-none ${sizeClasses} ${className}`}
      title={diagnostic ? `${company}: ${diagnostic}` : company}
      aria-label={`${company} initials`}
      data-logo-status={cacheUnavailable ? "cache_unavailable" : failedUrl === attemptKey ? "display_failed" : logo?.status || "missing_metadata"}
    >
      {getInitials(company)}
    </span>
  );
}
