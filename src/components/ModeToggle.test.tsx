import { describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@/test/utils";
import { ModeToggle } from "./ModeToggle";

describe("ModeToggle", () => {
	it("テーマを system から light に切り替える", () => {
		window.localStorage.removeItem("memo-breeze-theme");

		render(<ModeToggle />);
		const button = screen.getByRole("button", { name: /toggle theme/i });

		fireEvent.click(button);

		expect(window.localStorage.getItem("memo-breeze-theme")).toBe("light");
		expect(document.documentElement.classList.contains("light")).toBe(true);
	});

	it("テーマを light から dark に切り替える", () => {
		window.localStorage.setItem("memo-breeze-theme", "light");

		render(<ModeToggle />);
		const button = screen.getByRole("button", { name: /toggle theme/i });

		fireEvent.click(button);

		expect(window.localStorage.getItem("memo-breeze-theme")).toBe("dark");
		expect(document.documentElement.classList.contains("dark")).toBe(true);
	});
});
