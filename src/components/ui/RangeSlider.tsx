import React, { useCallback, useRef } from "react";

export interface RangeSliderProps {
  min: number;
  max: number;
  step?: number;
  value: [number, number];
  onChange: (value: [number, number]) => void;
  className?: string;
  disabled?: boolean;
}

export function RangeSlider({
  min,
  max,
  step = 1000,
  value,
  onChange,
  className = "",
  disabled = false,
}: RangeSliderProps) {
  const [minVal, maxVal] = value;
  const minValRef = useRef(minVal);
  const maxValRef = useRef(maxVal);
  minValRef.current = minVal;
  maxValRef.current = maxVal;

  const minPercent = Math.max(0, Math.min(100, ((minVal - min) / (max - min)) * 100));
  const maxPercent = Math.max(0, Math.min(100, ((maxVal - min) / (max - min)) * 100));

  const handleMinChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const val = Math.min(Number(e.target.value), maxValRef.current);
      onChange([val, maxValRef.current]);
    },
    [onChange]
  );

  const handleMaxChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const val = Math.max(Number(e.target.value), minValRef.current);
      onChange([minValRef.current, val]);
    },
    [onChange]
  );

  return (
    <div className={`relative flex items-center w-full h-8 select-none ${className}`}>
      {/* Background Track */}
      <div className="absolute w-full h-2 bg-[var(--color-paper-2)] border border-[var(--color-ink)] pointer-events-none" />

      {/* Active Range Bar */}
      <div
        className="absolute h-2 bg-[var(--color-ink)] pointer-events-none"
        style={{
          left: `${minPercent}%`,
          width: `${Math.max(0, maxPercent - minPercent)}%`,
        }}
      />

      {/* Low Thumb Input */}
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={minVal}
        disabled={disabled}
        onChange={handleMinChange}
        aria-label="Minimum value"
        className="thumb-range pointer-events-auto absolute w-full h-2 opacity-0 z-20 cursor-pointer appearance-none"
      />

      {/* High Thumb Input */}
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={maxVal}
        disabled={disabled}
        onChange={handleMaxChange}
        aria-label="Maximum value"
        className="thumb-range pointer-events-auto absolute w-full h-2 opacity-0 z-30 cursor-pointer appearance-none"
      />

      {/* Visible Low Thumb Marker */}
      <div
        className="absolute w-3.5 h-3.5 bg-[var(--color-ink)] border border-[var(--color-paper)] z-10 pointer-events-none -translate-x-1/2"
        style={{ left: `${minPercent}%` }}
      />

      {/* Visible High Thumb Marker */}
      <div
        className="absolute w-3.5 h-3.5 bg-[var(--color-ink)] border border-[var(--color-paper)] z-10 pointer-events-none -translate-x-1/2"
        style={{ left: `${maxPercent}%` }}
      />
    </div>
  );
}
