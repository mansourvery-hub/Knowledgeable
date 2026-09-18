# AGENTS.md

Read PRODUCT.md for product requirements.

Read MVP.md for current scope.

Read ARCHITECTURE.md for system architecture.

Read QUALITY.md for invariants.

Read TEST_STRATEGY.md for verification strategy.

Read IMPLEMENTATION_PLAN.md for current work.

Follow repository tooling and verification requirements.

Do not modify architectural constraints without updating
the authoritative architectural documentation.

## Operating Loop
1. Select the next READY task from IMPLEMENTATION_PLAN.md.
2. Identify smallest meaningful contracts (bricks).
3. Write small targeted tests.
4. Implement brick.
5. Verify (Local/`./verify`).
6. Update docs/state.
7. Commit/Push.
