import { describe, it, expect } from "vitest";
import { parseSalaryInput } from "../components/filters/SalaryRangeFilter";

describe("Salary Input Parser - parseSalaryInput", () => {
  it("parses standard integer inputs", () => {
    expect(parseSalaryInput("120000")).toBe(120000);
    expect(parseSalaryInput("85000")).toBe(85000);
    expect(parseSalaryInput("500000")).toBe(500000);
  });

  it("parses shorthand k notation", () => {
    expect(parseSalaryInput("150k")).toBe(150000);
    expect(parseSalaryInput("200K")).toBe(200000);
    expect(parseSalaryInput("125.5k")).toBe(125500);
    expect(parseSalaryInput("0k")).toBe(0);
    expect(parseSalaryInput("0.5k")).toBe(500);
  });

  it("strips dollar signs, commas, and whitespace", () => {
    expect(parseSalaryInput("$150,000")).toBe(150000);
    expect(parseSalaryInput("  $220,500  ")).toBe(220500);
    expect(parseSalaryInput("$175k")).toBe(175000);
    expect(parseSalaryInput(" $ 90,000 ")).toBe(90000);
  });

  it("interprets inputs <= 500 as thousands ($k)", () => {
    expect(parseSalaryInput("150")).toBe(150000);
    expect(parseSalaryInput("500")).toBe(500000);
    expect(parseSalaryInput("85")).toBe(85000);
    expect(parseSalaryInput("1")).toBe(1000);
    // Values > 500 are treated as raw amounts
    expect(parseSalaryInput("501")).toBe(501);
    expect(parseSalaryInput("1000")).toBe(1000);
  });

  it("handles zero gracefully", () => {
    expect(parseSalaryInput("0")).toBe(0);
    expect(parseSalaryInput("$0")).toBe(0);
    expect(parseSalaryInput(" 0 ")).toBe(0);
  });

  it("returns null for empty, whitespace, or invalid non-numeric inputs", () => {
    expect(parseSalaryInput("")).toBeNull();
    expect(parseSalaryInput("   ")).toBeNull();
    expect(parseSalaryInput("invalid")).toBeNull();
    expect(parseSalaryInput("abc k")).toBeNull();
    expect(parseSalaryInput("$$$")).toBeNull();
    expect(parseSalaryInput("NaN")).toBeNull();
  });
});
