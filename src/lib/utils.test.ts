import { describe, expect, it } from "vitest";
import { cn, formatTimestamp } from "./utils";

describe("cn", () => {
	it("クラス名を結合できる", () => {
		expect(cn("a", "b")).toBe("a b");
	});

	it("tailwindの競合クラスを解決できる", () => {
		expect(cn("px-2", "px-4")).toBe("px-4");
	});
});

describe("formatTimestamp", () => {
	it("秒数を mm:ss 形式でフォーマットする", () => {
		expect(formatTimestamp(0)).toBe("0:00");
		expect(formatTimestamp(65)).toBe("1:05");
		expect(formatTimestamp(90)).toBe("1:30");
	});

	it("3600秒以上は時を含めてフォーマットする", () => {
		expect(formatTimestamp(3600)).toBe("1:00:00");
		expect(formatTimestamp(3665)).toBe("1:01:05");
	});
});
