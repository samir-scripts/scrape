import type { Settings } from "../../../bindings/Settings";
import { Toggle } from "../../ui/Toggle";
import { Rule } from "../../ui/Rule";
import { Dither } from "../../ui/Dither";

interface AppearanceSettingsProps {
  settings: Settings;
  onUpdateSetting: (key: string, value: unknown) => void;
}

export function AppearanceSettings({
  settings,
  onUpdateSetting,
}: AppearanceSettingsProps) {
  const currentTheme = settings.ui_theme || "paper";

  const handleSelectTheme = (theme: "paper" | "obsidian") => {
    onUpdateSetting("ui.theme", theme);
    document.documentElement.setAttribute("data-theme", theme);
  };

  const handleToggleReduceMotion = (reduced: boolean) => {
    onUpdateSetting("ui.reduce_motion", reduced);
    document.documentElement.setAttribute(
      "data-reduce-motion",
      reduced ? "true" : "false"
    );
  };

  return (
    <div className="flex flex-col gap-6 w-full max-w-3xl select-none">
      <div>
        <h2 className="text-base font-black uppercase tracking-tight text-[var(--color-ink)]">
          APPEARANCE & BRUTALIST THEME
        </h2>
        <p className="text-xs font-mono text-[var(--color-ink-mute)]">
          HIGH-CONTRAST MONOCHROME PALETTES INSPIRED BY NIER:AUTOMATA & HALFTONE PRINT
        </p>
      </div>

      <Rule />

      {/* Theme Cards */}
      <div className="flex flex-col gap-3 font-mono text-xs">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          COLOR PALETTE PRESET
        </label>
        <div className="grid grid-cols-2 gap-4">
          {/* Paper Theme Card */}
          <div
            onClick={() => handleSelectTheme("paper")}
            className={`p-4 border-2 cursor-pointer flex flex-col gap-3 bg-[#F5F5DC] text-[#1A1A1A] ${
              currentTheme === "paper"
                ? "border-[var(--color-ink)]"
                : "border-[var(--color-ink-mute)]/50 opacity-70 hover:opacity-100"
            }`}
          >
            <div className="flex justify-between items-center font-bold">
              <span>PAPER (LIGHT BEIGE)</span>
              {currentTheme === "paper" && (
                <span className="bg-[#1A1A1A] text-[#F5F5DC] px-1.5 py-0.5 text-[10px]">
                  ACTIVE
                </span>
              )}
            </div>
            <p className="text-[11px] opacity-80">
              NieR-style aged paper background with high-contrast ink rules and halftone dot-matrix.
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
            onClick={() => handleSelectTheme("obsidian")}
            className={`p-4 border-2 cursor-pointer flex flex-col gap-3 bg-[#262628] text-[#FFF6EE] ${
              currentTheme === "obsidian"
                ? "border-[var(--color-ink)]"
                : "border-[var(--color-ink-mute)]/50 opacity-70 hover:opacity-100"
            }`}
          >
            <div className="flex justify-between items-center font-bold">
              <span>OBSIDIAN (DARK MONO)</span>
              {currentTheme === "obsidian" && (
                <span className="bg-[#FFF6EE] text-[#262628] px-1.5 py-0.5 text-[10px]">
                  ACTIVE
                </span>
              )}
            </div>
            <p className="text-[11px] opacity-80">
              Deep matte obsidian chassis with cream typography and CRT phosphor accents.
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

      {/* Motion and Animations */}
      <div className="flex flex-col gap-3 p-4 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/30">
        <Toggle
          checked={settings.ui_reduce_motion}
          onChange={handleToggleReduceMotion}
          label="REDUCE MOTION & TRANSITIONS"
          description="Disables all UI sliding bar transitions and selection animations (80-120ms) for instant rendering."
        />
      </div>

      <Rule />

      {/* Dither & Texture Previews */}
      <div className="flex flex-col gap-2 font-mono text-xs">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          DITHER & TEXTURE ENGINE PREVIEW
        </label>
        <div className="grid grid-cols-3 gap-3">
          <div className="border border-[var(--color-ink)] p-3 flex flex-col gap-2">
            <span className="text-[11px] font-bold">RADIAL DOT MATRIX (4PX)</span>
            <Dither type="dots" height="40px" className="opacity-80" />
          </div>
          <div className="border border-[var(--color-ink)] p-3 flex flex-col gap-2">
            <span className="text-[11px] font-bold">CONIC CHECKER (2PX)</span>
            <Dither type="checker" height="40px" className="opacity-80" />
          </div>
          <div className="border border-[var(--color-ink)] p-3 flex flex-col gap-2">
            <span className="text-[11px] font-bold">REPEATING 45° LINES</span>
            <Dither type="lines" height="40px" className="opacity-80" />
          </div>
        </div>
      </div>
    </div>
  );
}
