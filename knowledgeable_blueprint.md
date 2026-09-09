# Knowledgeable — Product & Architecture Blueprint
> **Status:** Foundational blueprint (master product specification).
> Agents: do not read this file by default. Start with `AGENTS.md` and the
> relevant subsystem document; consult this file only when they do not answer.
## 1. Executive Summary

**Knowledgeable** is an AI tutoring application whose core purpose is simple:

> **An AI tutor that maintains a model of what you understand about the world and uses that model to teach you things relevant to the edge of your understanding.**

The user can ask about essentially any subject, at any level, and converse naturally with an LLM tutor. Unlike a generic chatbot, the tutor does not treat every conversation as if the learner were starting from scratch. It consults a persistent, structured model of the learner's knowledge and uses it to construct explanations from first principles, beginning from concepts the learner already understands and introducing only the missing conceptual machinery needed to reach the requested subject.

Learning is the primary product loop. The knowledge graph is the persistent memory and structural model behind that loop, not the main activity or UI. After a useful learning interaction, the system extracts a small number of high-quality concepts and relationships into the learner's graph. Over time, the graph becomes a continuously maintained model of the learner's conceptual understanding.

The intended long-term effect is an **infinite learning machine**:

```text
conversation
    ↓
learning
    ↓
learner graph grows / strengthens
    ↓
better model of what the learner understands
    ↓
more relevant future explanations
    ↓
faster, more precise learning
    ↓
conversation
    ↺
```

The product should feel like **talking to a tutor who remembers how your mind works**, not like managing a second brain.

---

## 2. Product Philosophy

### 2.1 One job, done extremely well

The application should focus on one core capability:

> **Teach me something, starting from what I already understand.**

The product should resist feature creep. The graph browser, statistics, review system, visualizations, progress views, and other secondary features exist only insofar as they improve this central experience.

### 2.2 Understanding, not content consumption

The unit of progress is not:

- pages read
- lessons completed
- videos watched
- flashcards reviewed
- questions answered

The meaningful unit is:

> **a new piece of understanding becoming available to the learner.**

Graph growth and graph health are therefore useful representations of progress.

### 2.3 The learner's frontier

The default pedagogical strategy is **explanation from the learner's frontier**.

The tutor should begin from concepts that the learner already understands well and move toward the target through the smallest useful sequence of new concepts.

This is analogous to the idea of `i+1` in language learning, generalized to arbitrary knowledge domains.

### 2.4 First principles by default

The tutor should prefer explanations that can be grounded in simpler concepts already available in the learner's graph.

This does not mean every answer must be maximally long or literally descend to foundational mathematics. It means the system should prefer conceptual derivations over unexplained jumps whenever the learner's knowledge graph permits that approach.

### 2.5 Truth is invariant; representation is adaptive

A knowledge node represents a fundamentally true statement, definition, or other sufficiently reliable piece of knowledge. The learner-specific wording may evolve as the learner acquires more concepts.

Example progression:

```text
Early learner:
"Entropy measures how spread-out the possibilities are."

Later learner:
"Entropy measures uncertainty over a probability distribution."

Advanced learner:
"Entropy is the expected value of -log p(x) under p."
```

The underlying concept remains the same. The learner-facing representation becomes more information-dense as the learner's conceptual arsenal grows.

A key design objective is:

> **Minimum complexity subject to sufficient explanatory power for this learner.**

---

## 3. Core User Experience

The primary loop should be extremely simple:

```text
ASK
 ↓
UNDERSTAND THE LEARNER
 ↓
TEACH FROM THE FRONTIER
 ↓
OBSERVE UNDERSTANDING
 ↓
UPDATE THE LEARNER MODEL
 ↓
CRYSTALLIZE USEFUL NEW KNOWLEDGE
```

A typical interaction might look like:

> User: “Why does increasing interest rates reduce inflation?”

The tutor inspects the learner model and discovers that the learner already has strong understanding of prices, supply/demand, money, and basic economics, but has weaker understanding of monetary-policy transmission.

Instead of producing a generic explanation, it starts from that frontier:

> “You already understand that prices tend to rise when spending pressure exceeds available supply. The missing piece is what higher interest rates do to aggregate spending..."

The resulting conversation both teaches the topic and generates new evidence about what the learner understands.

---

## 4. Knowledge Graph

The graph is a **persistent learner knowledge model**.

It is not intended to be a note-taking system, wiki, or general-purpose second brain.

The learner generally has little interest in repeatedly reading definitions of concepts they already know. The graph is useful primarily because the tutor can reason over it. The user may inspect the topology when curious, but graph browsing is secondary.

### 4.1 Nodes

Each node represents a small, atomized, fundamentally true statement / definition / concept.

Example:

```text
Natural number
→ A whole number used for counting: 0, 1, 2, 3, ...
```

```text
JFK assassin
→ Lee Harvey Oswald
```

```text
Reason the sky is blue
→ Shorter wavelengths of sunlight are scattered more strongly by molecules in Earth's atmosphere.
```

```text
気に掛かる
→ 心配する
```

Nodes should be:

- small
- self-contained enough to be useful
- semantically precise
- easy for the learner to understand
- grounded in simpler concepts where practical
- phrased in a way compatible with the learner's current conceptual vocabulary
- high in information density without sacrificing comprehension

The user does **not** directly author or edit authoritative graph nodes. The learner supplies conversation and understanding; the tutor/graph-maintenance system proposes graph updates.

### 4.2 Two kinds of relationships

The graph contains two conceptually different relationship systems.

#### A. Semantic / reference links

These express what a concept refers to, uses, mentions, or is related to.

This is analogous to `[[concept]]` links in systems such as Logseq.

Example:

```text
Natural number
    └── uses / references → number
```

These links describe conceptual association.

#### B. Dependency links

These express what must be understood for another concept to be meaningfully understood.

Example:

```text
Natural number
    └── depends on → number
```

This graph is pedagogically critical.

A dependency edge answers:

> **What must be true in the learner's understanding for this concept to make sense?**

This enables first-principles teaching, gap detection, and graph repair.

### 4.3 Learner-specific node representation

A node has one underlying canonical truth, but its learner-facing wording is personalized.

Conceptually:

```text
Node
├── canonical statement / definition
├── learner-adapted representation
├── semantic links
├── dependency links
├── world confidence
└── learner confidence
```

The graph should not fork reality into different truths for different users. Personalization changes the explanatory representation, not the underlying truth claim.

---

## 5. Two Confidence Dimensions

Every graph node has two fundamentally different confidence values.

### 5.1 World confidence

`world_confidence ∈ [0, 1]`

Meaning:

> **How strongly supported is this statement by the world / available knowledge / expert consensus?**

World confidence is a **quality gate** for the graph.

The product should not knowingly populate the authoritative graph with questionable or poorly supported information. A configurable minimum threshold should exist; an example policy is:

```text
world_confidence < 0.80
    → do not add to authoritative graph
```

World confidence is therefore not a measure of whether the learner understands a statement. It measures whether the system considers the statement trustworthy enough to teach and retain as graph knowledge.

For example:

```text
"Lee Harvey Oswald killed JFK"
world_confidence ≈ 0.90
```

The exact calibration is domain- and evidence-dependent; the important architectural property is that low-confidence material is not silently converted into authoritative learner knowledge.

### 5.2 Learner confidence

`learner_confidence ∈ [0, 1]`

Meaning:

> **How confidently do we believe this learner currently understands this node?**

This is essentially the node's **health**.

The long-term objective is for nodes in the learner's graph to remain highly understood, for example around:

```text
learner_confidence >= 0.95
```

A node can lose learner confidence because:

1. natural forgetting / decay over long periods (years or decades), or
2. the tutor discovers during a later conversation that the learner's understanding is weaker, incomplete, or structurally dependent on a misconception.

Example:

```text
conditional probability
learner_confidence = 0.72
```

This indicates a weak node and should trigger targeted reinforcement rather than being ignored.

### 5.3 No giant mastery formula initially

The initial system should **not** combine dozens of variables into an elaborate mastery score.

Keep the core model intentionally simple:

```text
world_confidence
learner_confidence
```

Decay is a mechanism that modifies learner confidence over long periods. Additional scheduling or mastery dimensions can be introduced later only if real product evidence shows they are necessary.

---

## 6. Graph Health and Review

The ideal state of the learner graph is:

> **Everything in the graph is both trustworthy and well understood.**

If learner confidence drops below the desired health threshold, the system should identify the specific weak concepts and suggest or initiate a review.

For example:

```text
Graph health check

✓ number                 0.99
✓ function               0.98
✓ derivative             0.97
⚠ conditional probability 0.71
⚠ independence           0.83
```

A review should be granular and targeted. The system should not force the learner through a generic course or revisit everything.

### 6.1 Graph repair

Weak nodes can reveal structural problems beneath apparently advanced knowledge.

Example:

```text
quantum tunneling
      ↓
wave function
      ↓
probability amplitude
      ↓
complex numbers
```

If the learner appears to understand quantum tunneling but is shaky on probability amplitude, the tutor can temporarily descend the dependency graph and repair the weak prerequisite.

The repair process should go as deep as necessary toward healthy foundations.

This produces a **self-healing knowledge graph** rather than a static collection of facts.

---

## 7. Natural Decay

Learner confidence should decline naturally over long periods.

The intended behavior is closer to long-term memory decay than aggressive daily spaced repetition.

Examples:

- fundamental concepts should remain stable for very long periods when deeply understood
- peripheral facts may decay more noticeably
- a fact forgotten after ten years should become eligible for review even if it was once well learned

The initial implementation should keep this model deliberately simple. A baseline time-based decay function is sufficient.

More sophisticated scheduling logic should not be added unless actual usage proves the need.

---

## 8. The Tutor as a Graph-Navigating Agent

The LLM is not given the entire knowledge graph as one massive prompt.

Instead, it should interact with the learner graph as an external structured environment.

This is a central architectural idea.

### 8.1 Division of responsibility

**Deterministic application code owns state and invariants:**

- graph storage
- node identity
- relationship storage
- confidence values
- decay
- graph mutations
- world-confidence gate
- retrieval primitives
- consistency checks
- persistence

**The LLM owns reasoning:**

- interpreting the user's goal
- understanding what the learner appears to be asking
- selecting relevant knowledge
- deciding what to teach next
- choosing explanations
- detecting conceptual holes
- deciding when to descend dependencies
- proposing missing concepts
- proposing graph mutations

The guiding principle is:

> **Code owns state. LLM owns reasoning.**

### 8.2 Graph tools

The tutor should have a small set of operations it can call, conceptually such as:

```text
find_concept(topic)
get_concept(id)
get_dependencies(id)
get_related(id)
get_learner_confidence(id)
get_weak_concepts()
get_canonical_definition(id)
propose_concept(...)
propose_dependency(...)
propose_definition(...)
```

The exact API is an implementation choice. The conceptual requirement is that the LLM can inspect and navigate the learner's conceptual world instead of receiving an indiscriminate graph dump.

### 8.3 Why agentic graph access matters

For a request such as:

> “I want to understand quantum tunneling.”

the system may begin with no relevant neighboring nodes.

The tutor can then progressively reason:

```text
Target: quantum tunneling

What prerequisites matter?
→ wave function
→ energy
→ potential barrier
→ probability amplitude

Does learner know energy?
→ yes, 0.98

Does learner know wave function?
→ no

Teach wave function.

Now continue toward the target.
```

The tutor does not need to know the entire teaching path before the session starts. It can discover and navigate the path incrementally.

---

## 9. Dynamic Context Construction

A generic LLM cannot be expected to infer a good teaching context solely from a giant graph dump, and a conventional deterministic program cannot fully determine pedagogical relevance either.

The solution is a hybrid.

### 9.1 Deterministic retrieval layer

The application can mechanically retrieve a candidate region around the target:

```text
target concept
    ↓
semantic match
    ↓
nearby concepts
    ↓
dependency ancestors
    ↓
weak relevant concepts
```

This produces a manageable candidate set rather than attempting to decide the perfect teaching context.

### 9.2 LLM selection layer

The tutor receives:

- the user's current goal
- the relevant candidate graph region
- learner confidence for relevant concepts
- dependency information
- relevant learner representations
- graph invariants / constraints

It then decides which concepts actually matter for the current explanation.

Conceptually:

```text
GOAL
Explain quantum tunneling.

RELEVANT GRAPH REGION
...

LEARNER STATE
energy             0.98
probability        0.93
complex numbers    0.61
wave function      missing
...

TASK
Teach from the learner's current frontier.
Avoid re-explaining strong concepts unless necessary.
Repair weak prerequisites when they block understanding.
Prefer the smallest useful progression toward the target.
```

This hybrid model avoids both extremes:

- deterministic code trying to act as a human tutor
- an LLM hallucinating the entire learner graph from scratch

---

## 10. When the Graph Has No Relevant Knowledge

The system must work even for topics that are largely absent from the learner graph.

In that case the tutor enters a **graph construction / exploration mode**.

The LLM proposes the concepts needed to bridge the gap between the learner's known territory and the requested target.

Example:

```text
KNOWN
██████████████████████

           ↓

wave function
potential barrier
probability amplitude
quantum tunneling
```

The proposed concepts are not immediately authoritative graph entries.

They go through a separate graph-update / validation process.

This means the LLM is responsible for discovering useful pedagogical candidates, while the graph-maintenance system is responsible for determining whether those candidates are safe and appropriate to retain.

---

## 11. Knowledge Creation Pipeline

The conversation is **evidence**, not the graph itself.

The recommended conceptual pipeline is:

```text
User ↔ Tutor conversation
          ↓
observations about learning
          ↓
candidate learner-model updates
          ↓
proposed graph changes
          ↓
truth / consistency validation
          ↓
world-confidence gate
          ↓
graph mutation
```

### 11.1 The tutor should not directly mutate the authoritative graph

The conversational model should propose updates rather than directly write final graph truth.

For example:

```json
{
  "concept": "conditional probability",
  "candidate_statement": "The probability of A given B is how likely A is when B is known to be true.",
  "learner_observation": "Learner can apply the definition but confuses it with joint probability.",
  "candidate_confidence": 0.8
}
```

A graph-maintenance/validation process then evaluates and commits appropriate changes.

This separation protects the graph from conversational drift, hallucination, and accumulated noise.

---

## 12. Graph Growth

The graph grows organically from the learner's curiosity.

There is no requirement to construct a giant universal ontology in advance.

If the user becomes interested in economics, programming, Japanese, physics, or history, the graph grows into those regions as needed.

This means the long-term graph is:

```text
high-confidence canonical knowledge
+
learner-specific representations
+
learner-specific understanding state
+
relationships discovered through learning
```

rather than:

```text
an entire copy of human knowledge
```

The system only needs enough world knowledge to support the learner's current and emerging frontier.

---

## 13. Progress Model

The product should primarily treat graph growth and graph health as evidence of progress.

A session might produce an internal summary such as:

```text
+7 new concepts
+11 semantic links
+5 dependency edges
+8 concepts reinforced
1 weak prerequisite discovered and repaired
```

This is more meaningful than a generic “minutes studied” score.

The application does not need to expose all of this in the main UI. A lightweight indicator such as:

> **Understanding updated**
> +4 concepts · 2 reinforced · 1 weak dependency repaired

is sufficient for the core experience.

---

## 14. The Graph as a Secondary User Interface

The graph should be consultable but not central.

The normal user experience should primarily be conversation.

A reasonable mental model is:

```text
Main screen
└── conversation with tutor

Secondary views
├── knowledge graph
├── weak concepts / review
└── progress / topology
```

The user should not feel obligated to:

- maintain nodes
- manually tag concepts
- manually connect dependencies
- browse definitions of things they already know
- organize topics into folders
- maintain a personal taxonomy

The system does the maintenance automatically.

---

## 15. Emergent Graph Intelligence

As the graph matures, its topology becomes meaningful.

### 15.1 Foundational concepts

Nodes with large downstream dependency trees indicate foundational concepts.

For example:

```text
set
 ↓
functions
 ↓
probability
 ↓
statistics
 ↓
...
```

The exact structure is learner-specific.

### 15.2 Highly connected concepts

Some concepts will become hubs because they appear across many domains.

Examples might include:

- algorithm
- function
- number
- probability
- cause/effect

These highly connected concepts are valuable because a weakness in them can affect many otherwise unrelated areas.

### 15.3 Topology as intellectual biography

Over years, the learner's graph becomes a map of how their conceptual understanding developed.

This topology may be interesting to inspect, but it remains subordinate to the learning experience.

---

## 16. Handling Uncertainty

The initial product should avoid building an excessively complicated epistemology system.

The two confidence dimensions already provide a strong basis:

```text
world_confidence
learner_confidence
```

World confidence protects the quality of the graph.

Learner confidence represents the health of the learner's understanding.

Where knowledge is inherently uncertain, contested, interpretation-heavy, or probabilistic, the system should preserve appropriate uncertainty rather than presenting uncertain claims as absolute truth.

The general rule is:

> **The graph should strive to contain accurate knowledge, not merely confident-sounding statements.**

A mature implementation may later need richer epistemic metadata, but this should not be a prerequisite for the initial architecture.

---

## 17. Example End-to-End Session

### Starting state

The learner has strong knowledge of:

```text
number                  0.99
function                0.98
basic algebra            0.97
probability              0.96
```

and weak knowledge of:

```text
conditional probability  0.72
```

### User asks

> “Explain Bayes' theorem to me.”

### Tutor navigation

The tutor looks up the target and discovers dependencies such as:

```text
Bayes' theorem
    ↓
conditional probability
    ↓
probability
```

The learner already knows probability but conditional probability is weak.

### Tutor action

Rather than giving a generic Bayes-theorem lecture, the tutor repairs conditional probability first:

> “Before Bayes' theorem, there is one idea I want to tighten up: conditional probability...”

The tutor explains it using the learner's existing concepts and probes understanding.

### Learner model update

```text
conditional probability
0.72 → 0.96
```

### Continue

The tutor now teaches Bayes' theorem from the repaired frontier.

### Graph update

Potential new nodes might include:

```text
Bayes' theorem
posterior
prior
likelihood
```

with their appropriate semantic and dependency edges.

Only sufficiently trustworthy statements pass the world-confidence gate.

The session ends with the graph larger and healthier than before.

---

## 18. Long-Term Vision

The ideal outcome after years of use is not a massive database of notes.

It is a continuously maintained model of the learner's conceptual world.

The application should eventually be able to know things like:

```text
You understand these foundations extremely well.

These 3 concepts are currently weak.

This new subject is only two conceptual steps beyond
what you already know.

This explanation can therefore skip material you mastered
five years ago.

This advanced subject seems difficult only because one
foundational dependency has decayed.
```

The learner should increasingly experience the tutor as someone who **already knows what they know**.

That is the defining advantage over generic LLM assistants.

---

## 19. High-Level System Architecture

A conceptual architecture is:

```text
                         USER
                           │
                           ▼
                    ┌────────────┐
                    │   TUTOR    │
                    │    LLM     │
                    └─────┬──────┘
                          │
             graph navigation / reasoning
                          │
              ┌───────────┴───────────┐
              │                       │
              ▼                       ▼
     LEARNER GRAPH TOOLS      NEW KNOWLEDGE PROPOSALS
              │                       │
              └───────────┬───────────┘
                          ▼
                   GRAPH / MODEL LAYER
                          │
            ┌─────────────┼─────────────┐
            │             │             │
            ▼             ▼             ▼
         concepts      relations      confidence
            │             │             │
            └─────────────┼─────────────┘
                          ▼
                 PERSISTENT STORAGE
                          │
                          ▼
                LEARNER MODEL / GRAPH
                          │
                          └──────────────► future tutoring
```

A more complete logical decomposition is:

```text
1. Conversation Layer
   └── chat UI + streaming tutor interaction

2. Tutor / Reasoning Layer
   └── LLM + graph tools + pedagogical policy

3. Graph Access Layer
   └── retrieval, traversal, confidence lookup

4. Learner Model Layer
   └── learner-specific representations + confidence + decay

5. Graph Maintenance Layer
   └── candidate generation, validation, mutation

6. Knowledge Quality Layer
   └── world confidence + truth/consistency checks

7. Persistence Layer
   └── durable storage of graph and learner state
```

---

## 20. Important Architectural Invariants

These principles should remain true even as the implementation changes.

### Invariant 1 — The graph is not the UI

The graph exists primarily to improve tutoring.

### Invariant 2 — The graph is not chat history

Conversation is temporary evidence. Graph knowledge is curated persistent state.

### Invariant 3 — The LLM does not become the database

The graph and learner state must remain external, structured, and persistent.

### Invariant 4 — The LLM should not freely mutate authoritative truth

The tutor proposes. Validation and graph-maintenance logic commits.

### Invariant 5 — World confidence and learner confidence are different

One measures truthworthiness; the other measures understanding.

### Invariant 6 — Truth is stable; wording is adaptive

Personalization changes representation, not canonical truth.

### Invariant 7 — Teaching starts from the learner's frontier

Generic explanations are a fallback, not the default.

### Invariant 8 — Weak prerequisites are repaired

When an advanced concept depends on a shaky foundation, the tutor should descend the dependency graph and repair the foundation.

### Invariant 9 — Simplicity wins

Do not introduce elaborate mastery, scheduling, epistemology, or knowledge-management machinery until real use demonstrates that it is necessary.

### Invariant 10 — The application should feel like one thing

The user should experience:

> **“I ask something → the tutor understands where I am → it teaches me.”**

Everything else is infrastructure supporting that experience.

---

## 21. Suggested Initial Scope

The first version should validate the central loop rather than attempting the complete vision.

### Core MVP capabilities

1. Conversational tutor.
2. Persistent learner knowledge graph.
3. Small atomic nodes with canonical statements.
4. Semantic links and dependency links.
5. World confidence.
6. Learner confidence.
7. Simple long-term decay.
8. Graph retrieval tools exposed to the tutor.
9. Tutor-driven discovery of missing concepts.
10. Candidate graph updates with validation before persistence.
11. Detection of weak prerequisites during tutoring.
12. Targeted review of weak nodes.
13. Optional graph visualization / inspection.

### Explicitly not core initially

- elaborate spaced-repetition scheduling
- large manual note-taking system
- social features
- gamification-heavy progression
- complicated dashboards
- giant prebuilt universal ontology
- dozens of graph relation types
- extensive epistemic taxonomies
- user-authored authoritative nodes

These may become useful later, but they should not distract from proving the central hypothesis.

---

## 22. Platform Direction

The product is envisioned as:

- a web application
- a mobile application

The exact framework is intentionally not fixed in this blueprint.

The important architectural requirement is that the core learner model, tutoring logic, graph operations, and persistence remain independent of the presentation layer so that web and mobile clients can share the same conceptual system.

A practical implementation can therefore treat the client as a thin interaction layer around a shared backend/domain model.

---

## 23. The Core Hypothesis to Validate

The entire project ultimately rests on one product hypothesis:

> **An LLM tutor becomes substantially more useful when it has a persistent, structured, probabilistic model of what an individual learner understands and can use that model to teach from the learner's frontier.**

The graph is the mechanism that makes the hypothesis testable and persistent.

The success criterion is not whether the graph looks impressive.

It is whether users repeatedly experience:

> **“This tutor explains things to me in a way that makes sense because it actually knows what I already understand.”**

If that feeling becomes reliable, the product has succeeded at its core mission.

---

## 24. Product Mantra

> **The graph remembers.**  
> **The LLM reasons.**  
> **The learner learns.**

And the product's core promise:

> **Teach me anything, from what I already understand.**
