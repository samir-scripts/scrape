import { ensureVisibleCompanyLogo, loadCompanyLogoAsset } from "../../lib/companyLogoCache";
import { useEffect, useState } from "react";
import type { CompanyLogo as LogoReport } from "../../bindings/CompanyLogo";

export interface CompanyLogoProps {
  company: string;
  jobId?: string;
  sourceId?: string | null;
  logo?: LogoReport | null;
  size?: "sm" | "md" | "lg";
  className?: string;
}

export function CompanyLogo({
  company,
  jobId,
  sourceId,
  logo,
  size = "md",
  className = "",
}: CompanyLogoProps) {
  const [failedUrl, setFailedUrl] = useState<string | null>(null);
  const [cacheFailure, setCacheFailure] = useState<string | null>(null);
  const [cached, setCached] = useState<{reference: string; url: string} | null>(null);
  const [resolved, setResolved] = useState<{key: string; logo: LogoReport} | null>(null);
  const lookupKey = jobId && sourceId
    ? `${sourceId}\u0000${company.trim().toLowerCase()}\u0000${logo?.checked_at || "none"}`
    : "";

  useEffect(() => {
    let active=true;
    const reference=logo?.cached_asset_reference;
    if (reference) void loadCompanyLogoAsset(reference).then(url=>{if(active){setCached({reference,url});setCacheFailure(null);}}).catch(()=>{if(active){setCached(null);setCacheFailure(reference);}});
    return ()=>{active=false;};
  },[logo?.cached_asset_reference,logo?.checked_at]);

  useEffect(() => {
    let active = true;
    if (!jobId || !sourceId || (logo && logo.status !== "missing_metadata")) {
      return () => { active = false; };
    }
    void ensureVisibleCompanyLogo(jobId, sourceId, company, logo?.checked_at)
      .then(value => { if (active && value) setResolved({ key: lookupKey, logo: value }); })
      .catch(() => {});
    return () => { active = false; };
  }, [jobId, sourceId, company, logo?.status, logo?.checked_at, lookupKey]);

  const activeLogo = resolved?.key === lookupKey ? resolved.logo : logo;
  const sizeClasses = {
    sm: "w-[24px] h-[24px] text-xs",
    md: "w-[24px] h-[24px] text-xs",
    lg: "w-[32px] h-[32px] text-[13px]",
  }[size];

  const getInitials = (name: string): string => {
    if (!name) return "?";
    const clean = name.replace(/[^a-zA-Z0-9\s]/g, "").trim();
    const parts = clean.split(/\s+/);
    if (parts.length >= 2 && parts[0] && parts[1]) {
      return (parts[0][0] + parts[1][0]).toUpperCase();
    }
    return (clean.slice(0, 2) || "?").toUpperCase();
  };

  const derivedUrl = activeLogo?.asset || (cached?.reference === activeLogo?.cached_asset_reference ? cached?.url : null);
  const attemptKey = `${company}:${derivedUrl}:${activeLogo?.checked_at || "legacy"}`;
  const cacheUnavailable = !!activeLogo?.cached_asset_reference && cacheFailure === activeLogo.cached_asset_reference;
  const diagnostic = (cacheUnavailable ? "Cached employer logo could not be loaded; retry after refreshing." : activeLogo?.diagnostic) || (activeLogo?.status === "missing_metadata" ? "No employer logo metadata" : activeLogo?.status === "download_failed" ? "Employer logo download failed" : activeLogo?.status === "decode_failed" ? "Employer logo decoding failed" : "");

  if (derivedUrl && failedUrl !== attemptKey) {
    return (
      <span
        className={`inline-flex items-center justify-center border border-[var(--color-ink)] bg-[var(--color-logo-surface)] shrink-0 overflow-hidden ${sizeClasses} ${className}`}
        title={diagnostic ? `${company}: ${diagnostic}` : company}
      >
        <img
          src={derivedUrl}
          data-logo-status={activeLogo?.status || "verified"}
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
      data-logo-status={cacheUnavailable ? "cache_unavailable" : failedUrl === attemptKey ? "display_failed" : activeLogo?.status || "missing_metadata"}
    >
      {getInitials(company)}
    </span>
  );
}
