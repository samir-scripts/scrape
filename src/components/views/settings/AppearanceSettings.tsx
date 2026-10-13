import type { Settings } from "../../../bindings/Settings";
import { Toggle } from "../../ui/Toggle";
import { Rule } from "../../ui/Rule";
import { Dither } from "../../ui/Dither";
import { useUIStore } from "../../../lib/store";
import { FONT_OPTIONS, type FontFamilyId } from "../../../lib/constants";

interface AppearanceSettingsProps {
  settings: Settings;
  onUpdateSetting: (key: string, value: unknown) => void;
}

export function AppearanceSettings({
  settings,
  onUpdateSetting,
}: AppearanceSettingsProps) {
  const currentTheme = settings.ui_theme || "paper";
  const currentFont = (settings.ui_font_family as FontFamilyId) || "default";

  const handleSelectTheme = (theme: "paper" | "obsidian") => {
    onUpdateSetting("ui.theme", theme);
    document.documentElement.setAttribute("data-theme", theme);
  };

  const handleSelectFont = (fontId: FontFamilyId) => {
    onUpdateSetting("ui.font_family", fontId);
    document.documentElement.setAttribute("data-font", fontId);
    try {
      localStorage.setItem("scrape:font_family", fontId);
    } catch {}
  };

  const handleToggleTooltips = (enabled: boolean) => {
    onUpdateSetting("ui.tooltips_enabled", enabled);
    useUIStore.getState().setTooltipsEnabled(enabled);
  };

  const handleToggleReduceMotion = (reduced: boolean) => {
    onUpdateSetting("ui.reduce_motion", reduced);
    document.documentElement.setAttribute(
      "data-reduce-motion",
      reduced ? "true" : "false",
    );
  };

  return (
    <div className="flex flex-col gap-6 w-full max-w-3xl select-none">
      <div>
        <h2 className="text-base font-black uppercase tracking-tight text-[var(--color-ink)]">
          APPEARANCE
        </h2>
      </div>

      <Rule />

      {/* Theme Cards */}
      <div className="flex flex-col gap-3 font-mono text-xs">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          THEME
        </label>
        <div className="grid grid-cols-2 gap-4">
          {/* Paper Theme Card */}
          <div
            role="button"
            tabIndex={0}
            onClick={() => handleSelectTheme("paper")}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                handleSelectTheme("paper");
              }
            }}
            className={`p-4 border-2 cursor-pointer flex flex-col gap-3 bg-[#F5F5DC] text-[var(--color-accent-ink)] focus:outline-none ${
              currentTheme === "paper"
                ? "border-[var(--color-ink)]"
                : "border-[var(--color-ink-mute)]/50"
            }`}
          >
            <div className="flex justify-between items-center font-bold">
              <span>PAPER</span>
              {currentTheme === "paper" && (
                <span className="bg-[#1A1A1A] text-[#F5F5DC] px-1.5 py-0.5 text-xs">
                  ACTIVE
                </span>
              )}
            </div>
            <p className="text-xs opacity-80">
              Light background with contrast ink rules.
            </p>
            <div className="flex gap-1.5 pt-2 border-t border-[#1A1A1A]">
              <span className="w-5 h-5 bg-[#F5F5DC] border border-[#1A1A1A]" />
              <span className="w-5 h-5 bg-[#E6E2C8] border border-[#1A1A1A]" />
              <span className="w-5 h-5 bg-[#1A1A1A]" />
              <span className="w-5 h-5 bg-[#C6C2F2]" />
              <span className="w-5 h-5 bg-[#D6DC82]" />
              <span className="w-5 h-5 bg-[#D86072]" />
            </div>
          </div>

          {/* Obsidian Theme Card */}
          <div
            role="button"
            tabIndex={0}
            onClick={() => handleSelectTheme("obsidian")}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                handleSelectTheme("obsidian");
              }
            }}
            className={`p-4 border-2 cursor-pointer flex flex-col gap-3 bg-[#262628] text-[#FFF6EE] focus:outline-none ${
              currentTheme === "obsidian"
                ? "border-[var(--color-ink)]"
                : "border-[var(--color-ink-mute)]/50"
            }`}
          >
            <div className="flex justify-between items-center font-bold">
              <span>OBSIDIAN</span>
              {currentTheme === "obsidian" && (
                <span className="bg-[#FFF6EE] text-[#262628] px-1.5 py-0.5 text-xs">
                  ACTIVE
                </span>
              )}
            </div>
            <p className="text-xs opacity-80">
              Dark background with high-contrast text.
            </p>
            <div className="flex gap-1.5 pt-2 border-t border-[#FFF6EE]">
              <span className="w-5 h-5 bg-[#262628] border border-[#FFF6EE]" />
              <span className="w-5 h-5 bg-[#36363a] border border-[#FFF6EE]" />
              <span className="w-5 h-5 bg-[#FFF6EE]" />
              <span className="w-5 h-5 bg-[#C6C2F2]" />
              <span className="w-5 h-5 bg-[#D6DC82]" />
              <span className="w-5 h-5 bg-[#D86072]" />
            </div>
          </div>
        </div>
      </div>

      <Rule />

      {/* Typography & Font Family Cards */}
      <div className="flex flex-col gap-3 text-xs">
        <div className="flex justify-between items-center font-mono">
          <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
            TYPOGRAPHY & FONT FAMILY
          </label>
          <span className="text-xs text-[var(--color-ink-mute)]">
            ACTIVE: {FONT_OPTIONS.find((f) => f.id === currentFont)?.name || "DEFAULT"}
          </span>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {FONT_OPTIONS.map((font) => {
            const isSelected = currentFont === font.id;

            return (
              <div
                key={font.id}
                role="button"
                tabIndex={0}
                onClick={() => handleSelectFont(font.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    handleSelectFont(font.id);
                  }
                }}
                className={`p-3 border-2 cursor-pointer flex flex-col justify-between gap-2.5 bg-[var(--color-paper)] text-[var(--color-ink)] focus:outline-none transition-none ${
                  isSelected
                    ? "border-[var(--color-ink)] bg-[var(--color-paper-2)]/60"
                    : "border-[var(--color-ink-mute)]/30 hover:border-[var(--color-ink)]"
                }`}
              >
                <div className="flex flex-col gap-1 font-mono">
                  <div className="flex justify-between items-center font-bold">
                    <span className="text-xs">{font.label}</span>
                    {isSelected && (
                      <span className="bg-[var(--color-ink)] text-[var(--color-paper)] px-1.5 py-0.5 text-[9px]">
                        ACTIVE
                      </span>
                    )}
                  </div>
                  <span className="text-xs text-[var(--color-ink-mute)]">
                    {font.name} · {font.category === "readable" ? "READABLE SANS" : "MONOSPACE"}
                  </span>
                  <p className="text-xs opacity-80 leading-snug pt-1">
                    {font.description}
                  </p>
                </div>

                {/* Dedicated Typeface Specimen Box (Displays its own font regardless of active application theme/font) */}
                <div
                  className={`font-specimen font-specimen-${font.id} mt-2 p-2.5 border border-[var(--color-ink)]/30 bg-[var(--color-paper-2)]/40 flex flex-col gap-1.5 select-none`}
                >
                  <div className="flex justify-between items-baseline border-b border-[var(--color-ink)]/15 pb-1">
                    <span className="text-xs font-bold tracking-normal">
                      AaBbCc 0123456789
                    </span>
                    <span className="text-xs opacity-80">
                      {font.category === "readable" ? "ABC xyz" : "{ } => !="}
                    </span>
                  </div>
                  <div className="text-xs leading-tight break-words">
                    {font.previewText}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <Rule />

      {/* Interface & Motion Controls */}
      <div className="flex flex-col gap-4 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
        <Toggle
          checked={settings.ui_tooltips_enabled ?? true}
          onChange={handleToggleTooltips}
          label="GLOBAL TOOLTIPS"
          description="Displays contextual hints on hover across navigation tabs, table headers, and action buttons."
        />

        <div className="border-t border-[var(--color-ink)]/20" />

        <Toggle
          checked={settings.ui_reduce_motion}
          onChange={handleToggleReduceMotion}
          label="REDUCE MOTION"
          description="Disables transitions and animations for instant rendering."
        />
      </div>

      <Rule />

      {/* Dither & Texture Previews */}
      <div className="flex flex-col gap-2 font-mono text-xs">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          PATTERNS
        </label>
        <div className="grid grid-cols-3 gap-3">
          <div className="border border-[var(--color-ink)] p-3 flex flex-col gap-2">
            <span className="text-xs font-bold">
              RADIAL DOT MATRIX (4PX)
            </span>
            <Dither type="dots" height="40px" className="opacity-80" />
          </div>
          <div className="border border-[var(--color-ink)] p-3 flex flex-col gap-2">
            <span className="text-xs font-bold">CONIC CHECKER (2PX)</span>
            <Dither type="checker" height="40px" className="opacity-80" />
          </div>
          <div className="border border-[var(--color-ink)] p-3 flex flex-col gap-2">
            <span className="text-xs font-bold">REPEATING 45° LINES</span>
            <Dither type="lines" height="40px" className="opacity-80" />
          </div>
        </div>
      </div>
    </div>
  );
}
