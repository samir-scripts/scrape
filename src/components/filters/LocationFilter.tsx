import { useState, useEffect } from "react";
import { api } from "../../lib/api";

export interface LocationFilterProps {
  countries: string[] | null;
  regions: string[] | null;
  remote: string | null;
  includeUnknown: boolean;
  onChange: (vals: {
    countries: string[] | null;
    regions: string[] | null;
    remote: string | null;
    includeUnknown: boolean;
  }) => void;
}

const COMMON_COUNTRIES = [
  "United States",
  "Canada",
  "United Kingdom",
  "Germany",
  "France",
  "India",
];

const COMMON_US_STATES = [
  "CA", "NY", "TX", "WA", "MA", "IL", "CO", "NC", "VA", "FL", "GA"
];

const REMOTE_OPTIONS: { id: string; label: string }[] = [
  { id: "all", label: "ANY WORKPLACE" },
  { id: "remote", label: "REMOTE ONLY" },
  { id: "hybrid", label: "HYBRID ONLY" },
  { id: "onsite", label: "ONSITE ONLY" },
];

export function LocationFilter({
  countries,
  regions,
  remote,
  includeUnknown,
  onChange,
}: LocationFilterProps) {
  const [availableCountries, setAvailableCountries] = useState<string[]>(COMMON_COUNTRIES);
  const [availableRegions, setAvailableRegions] = useState<string[]>(COMMON_US_STATES);
  const [countryInput, setCountryInput] = useState("");
  const [regionInput, setRegionInput] = useState("");

  useEffect(() => {
    api.listLocationOptions()
      .then((opts) => {
        if (opts.countries && opts.countries.length > 0) {
          const merged = Array.from(new Set([...COMMON_COUNTRIES, ...opts.countries]));
          setAvailableCountries(merged);
        }
        if (opts.regions && opts.regions.length > 0) {
          const merged = Array.from(new Set([...COMMON_US_STATES, ...opts.regions]));
          setAvailableRegions(merged);
        }
      })
      .catch(() => {
        // Fallback to static defaults
      });
  }, []);

  const selectedCountries = countries || [];
  const selectedRegions = regions || [];

  const handleToggleCountry = (country: string) => {
    let next: string[];
    if (selectedCountries.includes(country)) {
      next = selectedCountries.filter((c) => c !== country);
    } else {
      next = [...selectedCountries, country];
    }
    onChange({
      countries: next.length > 0 ? next : null,
      regions,
      remote,
      includeUnknown,
    });
  };

  const handleToggleRegion = (reg: string) => {
    let next: string[];
    if (selectedRegions.includes(reg)) {
      next = selectedRegions.filter((r) => r !== reg);
    } else {
      next = [...selectedRegions, reg];
    }
    onChange({
      countries,
      regions: next.length > 0 ? next : null,
      remote,
      includeUnknown,
    });
  };

  const handleAddCustomCountry = () => {
    const trimmed = countryInput.trim();
    if (!trimmed) return;
    if (!selectedCountries.includes(trimmed)) {
      onChange({
        countries: [...selectedCountries, trimmed],
        regions,
        remote,
        includeUnknown,
      });
    }
    setCountryInput("");
  };

  const handleAddCustomRegion = () => {
    const trimmed = regionInput.trim().toUpperCase();
    if (!trimmed) return;
    if (!selectedRegions.includes(trimmed)) {
      onChange({
        countries,
        regions: [...selectedRegions, trimmed],
        remote,
        includeUnknown,
      });
    }
    setRegionInput("");
  };

  const hasFilter =
    (countries != null && countries.length > 0) ||
    (regions != null && regions.length > 0) ||
    (remote != null && remote !== "all");

  return (
    <div className="flex flex-col gap-3 font-mono text-xs select-none">
      <div className="flex justify-between items-center">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          GEOGRAPHY & REMOTE POLICY
        </label>
        {hasFilter && (
          <button
            onClick={() =>
              onChange({
                countries: null,
                regions: null,
                remote: "all",
                includeUnknown,
              })
            }
            className="text-[10px] text-[var(--color-ink-mute)] hover:text-[var(--color-cherry)] uppercase cursor-pointer"
          >
            [RESET]
          </button>
        )}
      </div>

      {/* Workplace Mode (Remote / Hybrid / Onsite) */}
      <div className="flex flex-col gap-1">
        <span className="text-[10px] text-[var(--color-ink-mute)] uppercase font-bold">
          WORKPLACE POLICY
        </span>
        <div className="grid grid-cols-2 gap-1.5">
          {REMOTE_OPTIONS.map((opt) => {
            const active = (remote || "all") === opt.id;
            return (
              <button
                key={opt.id}
                type="button"
                onClick={() =>
                  onChange({
                    countries,
                    regions,
                    remote: opt.id === "all" ? null : opt.id,
                    includeUnknown,
                  })
                }
                className={`py-1 px-2 text-[10px] font-bold uppercase tracking-wider border cursor-pointer ${
                  active
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                {opt.label}
              </button>
            );
          })}
        </div>
      </div>

      {/* Target Countries */}
      <div className="flex flex-col gap-1 pt-1">
        <span className="text-[10px] text-[var(--color-ink-mute)] uppercase font-bold">
          COUNTRIES
        </span>
        <div className="flex flex-wrap gap-1">
          {availableCountries.slice(0, 8).map((c) => {
            const active = selectedCountries.includes(c);
            return (
              <button
                key={c}
                type="button"
                onClick={() => handleToggleCountry(c)}
                className={`px-2 py-0.5 text-[10px] uppercase font-bold border cursor-pointer ${
                  active
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                {c}
              </button>
            );
          })}
        </div>
        <div className="flex gap-1.5 mt-1">
          <input
            type="text"
            value={countryInput}
            placeholder="ADD COUNTRY (E.G. JAPAN, GERMANY)..."
            onChange={(e) => setCountryInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleAddCustomCountry()}
            className="flex-1 px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] text-xs uppercase focus:outline-none"
          />
          <button
            type="button"
            onClick={handleAddCustomCountry}
            className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] text-[10px] font-bold uppercase cursor-pointer"
          >
            + ADD
          </button>
        </div>
      </div>

      {/* Target States / Regions */}
      <div className="flex flex-col gap-1 pt-1">
        <span className="text-[10px] text-[var(--color-ink-mute)] uppercase font-bold">
          STATES & PROVINCES
        </span>
        <div className="flex flex-wrap gap-1">
          {availableRegions.slice(0, 12).map((r) => {
            const active = selectedRegions.includes(r);
            return (
              <button
                key={r}
                type="button"
                onClick={() => handleToggleRegion(r)}
                className={`w-7 h-6 flex items-center justify-center text-[10px] font-bold border cursor-pointer ${
                  active
                    ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                    : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
                }`}
              >
                {r}
              </button>
            );
          })}
        </div>
        <div className="flex gap-1.5 mt-1">
          <input
            type="text"
            value={regionInput}
            placeholder="STATE/PROV CODE (E.G. NY, TX, ON)..."
            onChange={(e) => setRegionInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleAddCustomRegion()}
            className="flex-1 px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] text-xs uppercase focus:outline-none"
          />
          <button
            type="button"
            onClick={handleAddCustomRegion}
            className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-ink)] text-[var(--color-paper)] text-[10px] font-bold uppercase cursor-pointer"
          >
            + ADD
          </button>
        </div>
      </div>

      {/* Include Unknown Toggle */}
      <div className="flex items-center gap-2 pt-1">
        <label className="flex items-center gap-2 cursor-pointer text-[11px] text-[var(--color-ink)]">
          <input
            type="checkbox"
            checked={includeUnknown}
            onChange={(e) =>
              onChange({
                countries,
                regions,
                remote,
                includeUnknown: e.target.checked,
              })
            }
            className="w-3.5 h-3.5 border border-[var(--color-ink)] bg-[var(--color-paper)] rounded-none accent-[var(--color-ink)] cursor-pointer"
          />
          <span className="uppercase font-bold tracking-wider">
            INCLUDE POSTINGS WITHOUT PARSED LOCATION
          </span>
        </label>
      </div>
    </div>
  );
}
