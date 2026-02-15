import { describe, expect, it } from "vitest";
import { render, screen } from "@/test/utils";
import { LanguageSwitcher } from "./LanguageSwitcher";

describe("LanguageSwitcher", () => {
	it("言語選択のUIを表示する", () => {
		render(<LanguageSwitcher />);
		expect(screen.getByRole("combobox")).toBeInTheDocument();
	});
});
