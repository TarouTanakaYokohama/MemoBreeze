## Skills
A skill is a set of local instructions stored in a `SKILL.md` file.

### Available skills in this repository
- multi-agent-orchestrator: Hierarchical multi-agent orchestration for Codex CLI with supervisor/reviewer/workers, dry-run/apply modes, review gates, and artifact generation. (file: /Users/tarou/Desktop/MemoBreeze/.agents/skills/multi-agent-orchestrator/SKILL.md)
- find-skills: Helps discover and install skills when users ask for capabilities or extensions. (file: /Users/tarou/Desktop/MemoBreeze/.agents/skills/find-skills/SKILL.md)
- vercel-composition-patterns: React composition architecture and refactoring guidance. (file: /Users/tarou/Desktop/MemoBreeze/.agents/skills/vercel-composition-patterns/SKILL.md)
- vercel-react-best-practices: React/Next.js performance best practices. (file: /Users/tarou/Desktop/MemoBreeze/.agents/skills/vercel-react-best-practices/SKILL.md)
- web-design-guidelines: Web UI/UX and accessibility review guidelines. (file: /Users/tarou/Desktop/MemoBreeze/.agents/skills/web-design-guidelines/SKILL.md)

### Trigger rules
- Use a skill when the user explicitly names it with `$skill-name` or plain text.
- Use a skill when the task clearly matches the skill description.
- If multiple skills apply, use the smallest necessary set and state the order.
- If a named skill is missing or unreadable, state that briefly and continue with best fallback.

### Usage rules
1. Open the selected `SKILL.md` first and read only what is needed.
2. Resolve relative paths from the skill directory first.
3. Load only required reference files from `references/`.
4. Prefer bundled `scripts/` and `assets/` over recreating logic.
5. Keep context small and avoid unnecessary file loading.
