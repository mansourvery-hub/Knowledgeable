# IMPLEMENTATION_PLAN.md

## Implementation Graph

```
                    ┌───────────────┐
                    │ Project shell │
                    └───────┬───────┘
                            │
                     ┌──────┴──────┐
                     ↓             ↓
              Data model       Chat Controller
                     │             │
                     ↓             ↓
               Repository      Tooling (LLM)
                     │             │
                     └──────┬──────┘
                            ↓
                       E2E Validation
```

## Current Task Status

| ID | Description | Dependencies | Status |
| :--- | :--- | :--- | :--- |
| T1 | Reactive Tools implementation | - | COMPLETE |
| T2 | SSE Stream robustness | - | COMPLETE |
| T3 | E2E Playwright setup | - | COMPLETE |
| T4 | CORS hardening | - | COMPLETE |
| T5 | Manual E2E Validation | T3 | COMPLETE |
| T7 | Fix Tool Calling Engine | T1 | COMPLETE |
| T8 | Graph neighborhood API (`GET /v1/graph/neighborhood`, bounded CTE + learner confidence + 503) | - | COMPLETE |

## Next Steps
- T9 (READY): Graph Visualizer frontend — neighborhood view consuming `/v1/graph/neighborhood` (semantic vs dependency distinction, confidence visibility, review/weak view; chat remains primary).
