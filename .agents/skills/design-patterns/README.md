# Design Patterns Skill

Analyze JS/TS codebases to detect GoF (Gang of Four) patterns, suggest stack-native refactoring options, and evaluate implementation quality against SOLID principles.

## When To Use

Use this skill when you need to:
- Trace pattern usage in code to understand architecture.
- Identify design smells (e.g., God Class, tight coupling) and find patterns to resolve them.
- Refactor legacy or complex files using stack-native patterns.
- Evaluate the quality of custom pattern implementations.

**Do not use for:** Codebase-wide onboarding or repository-level style guides. Use [codebase-documenter](../codebase-documenter/README.md) or [coding-rules-generator](../coding-rules-generator/README.md) instead.

## Operating Modes

Choose one of three modes to run this skill:

| Mode | Input / Trigger | Output | Best For |
|------|-----------------|--------|----------|
| `detect` | Directory or file path | JSON report with signatures, files, and confidence score | Cataloging pattern usage across a project or dependency graph |
| `suggest` | Legacy file, class, or refactoring request | Refactoring proposal with code snippets and rationale | Planning architectural updates, decoupling classes |
| `evaluate` | Sibling file or implementation | Markdown evaluation of correctness, SOLID adherence, and edge-cases | Code reviews, testing robustness, validating extensions |

## Available Context

- `signatures/detection-rules.yaml` — Rules for structural, behavioral, and creational patterns.
- `signatures/code-smells.yaml` — Structural indicators of design smells.
- `signatures/stack-patterns.yaml` — Idiomatic patterns native to specific frameworks (NestJS DI, React Hooks, RxJS streams).
- `reference/` (`creational.md`, `structural.md`, `behavioral.md`) — Reference implementations and typical pitfalls for GoF patterns.
- `checklists/pattern-evaluation.md` — Checklist for verifying implementation quality.

## Step-by-Step Flow

### Detect
1. Run stack detection via project files (`package.json`, `tsconfig.json`).
2. Search files for design pattern signatures.
3. Classify patterns as stack-native or custom.
4. Calculate a confidence score (0.0 - 1.0) and output a JSON summary.

### Suggest
1. Identify architectural design smells in code.
2. Select target patterns suited for the stack and requirements.
3. Draft the refactoring, showing how client code interacts with the proposed structure.
4. Detail trade-offs.

### Evaluate
1. Contrast the implementation against reference standards.
2. Check interface constraints and exception handling.
3. Rate compliance with SOLID principles.
4. Provide refactoring checklist items to fix shortcomings.

## Example Usage

**Detect Patterns:**
> *"Use design-patterns in detect mode on src/ to find implemented GoF patterns."*

**Refactoring Suggestion:**
> *"Use design-patterns in suggest mode to refactor src/order-service.ts to handle payment gateways using the Strategy pattern."*

**Evaluate Quality:**
> *"Use design-patterns in evaluate mode to check our custom Observer pattern in src/events.ts."*

See the [Agent Architect Toolkit Skills Guide](../../SKILLS-GUIDE.md#7-design-patterns) for complete details.
