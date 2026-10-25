import { api } from "./api";
import type { CompanyLogo } from "../bindings/CompanyLogo";
import { isDemoMode } from "./runtime";

const assets = new Map<string, Promise<string>>();
const logoLookups = new Map<string, { promise: Promise<CompanyLogo | null>; expiresAt: number | null }>();
const negativeLookupTtlMs = 10 * 60 * 1000;

export function loadCompanyLogoAsset(reference: string): Promise<string> {
  let value = assets.get(reference);
  if (!value) {
    value = api.getCompanyLogoAsset(reference).catch(error => { assets.delete(reference); throw error; });
    assets.set(reference, value);
    if (assets.size > 64) assets.delete(assets.keys().next().value!);
  }
  return value;
}

export function ensureVisibleCompanyLogo(
  jobId: string,
  sourceId: string,
  company: string,
  checkedAt: string | null | undefined,
): Promise<CompanyLogo | null> {
  if (isDemoMode()) return Promise.resolve(null);
  const identity = `${sourceId}\u0000${company.trim().toLowerCase()}\u0000${checkedAt || "none"}`;
  const now = Date.now();
  const existing = logoLookups.get(identity);
  if (existing && (existing.expiresAt === null || existing.expiresAt > now)) {
    return existing.promise;
  }
  if (existing) logoLookups.delete(identity);
  const promise = api.ensureCompanyLogoForJob(jobId);
  const entry = { promise, expiresAt: null as number | null };
  logoLookups.set(identity, entry);
  void promise.then(result => {
    if (logoLookups.get(identity) === entry && result === null) {
      entry.expiresAt = Date.now() + negativeLookupTtlMs;
    }
  }, () => {
    if (logoLookups.get(identity) === entry) {
      entry.expiresAt = Date.now() + negativeLookupTtlMs;
    }
  });
  if (logoLookups.size > 64) logoLookups.delete(logoLookups.keys().next().value!);
  return promise;
}

export function clearCompanyLogoCache() {
  assets.clear();
  logoLookups.clear();
}
