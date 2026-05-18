# Create Anyrun Plugin Skill — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Create a `create-anyrun-plugin` skill at `.opencode/skills/create-anyrun-plugin/SKILL.md` that guides AI-assisted scaffolding of new anyrun plugins.

**Architecture:** Single SKILL.md with embedded workflows, decision flowchart, and 3 code templates (simple stateless, state+fuzzy, complex state). AI gathers plugin requirements → scaffolds files from templates → registers in workspace → verifies.

**Tech Stack:** Markdown, Rust (anyrun plugin SDK), RON config format

---

### Task 1: Create `.opencode/skills/create-anyrun-plugin/SKILL.md`

- [ ] **Write SKILL.md**

Write the file at `.opencode/skills/create-anyrun-plugin/SKILL.md` containing:
- YAML frontmatter with `name: create-anyrun-plugin` and `description: Use when...` trigger conditions
- Overview + prerequisites (workspace conventions from AGENTS.md)
- Workflow: Gather info → Choose template → Scaffold files → Register in workspace → Verify
- Decision matrix: which template fits the plugin's needs
- 3 template variants as code blocks
- Verification checklist
- Common mistakes section

- [ ] **Verify the file exists**

Run: `ls -la .opencode/skills/create-anyrun-plugin/SKILL.md`

- [ ] **Commit**

```bash
git add .opencode/skills/create-anyrun-plugin/SKILL.md .opencode/plans/2026-05-18-create-anyrun-plugin-skill.md
git commit -m "feat: add create-anyrun-plugin skill for scaffolding new plugins"
```
