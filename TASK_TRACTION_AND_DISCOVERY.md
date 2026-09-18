# TASK: Future Traction & Discovery Exploration

**Status:** Optional / Deferred / Do Not Execute Automatically
**Priority:** Low until the core product is released and real users provide feedback
**Scope:** Product discovery, documentation, visibility, community outreach, and user acquisition through organic awareness
**Does NOT authorize:** Core architecture refactors, scoring changes, speculative feature development, monetization work, or disruptive changes to the stable release

---

## 0. Purpose

This document preserves ideas for getting CompreDef known after the core project is considered stable.

The goal is **not monetization** and not growth-at-all-costs. The goal is simply:

> Get the right Japanese-learning / Anki / Yomitan users to discover that CompreDef exists, immediately understand what it solves, and try it.

These are exploration ideas, not commitments. Future work should be driven by evidence from actual users rather than by implementing everything listed here.

---

# 1. Product Positioning

## 1.1 Do NOT position CompreDef as “another Japanese dictionary”

The project should avoid competing directly with Yomitan's core identity or with generic dictionary add-ons.

The strongest positioning is complementary:

> **Yomitan finds the definitions. CompreDef decides which definition belongs on your card.**

Alternative explanatory framing:

> **CompreDef automatically chooses the Japanese definition you are most likely to understand, using what you already know from Anki.**

The product is therefore closer to a **learner-aware definition selector/ranker** than a dictionary.

## 1.2 The problem to communicate

A learner can install several excellent Japanese dictionaries and still face a practical problem:

- multiple legitimate definitions may exist;
- some are too difficult for the learner;
- some are overly specialized;
- some contain vocabulary/kanji the learner does not know;
- the learner may have to manually inspect several candidates every time they mine a word.

CompreDef's value proposition is reducing that repeated judgment step.

## 1.3 Strong emotional/use-case hook

A particularly understandable use case is the transition from English definitions toward Japanese monolingual definitions:

> **Move toward monolingual Japanese definitions gradually, without manually judging every definition you encounter.**

This is a more concrete story than “AI dictionary ranking.”

---

# 2. Primary Audience to Explore

The first audience should be narrow rather than “everyone who uses Anki.”

### Primary

- Japanese learners using Anki for sentence/word mining
- Yomitan users who already have several Japanese dictionaries installed
- learners attempting a gradual monolingual-dictionary transition
- AJATT / immersion-oriented learners
- users who already understand the Anki + Yomitan workflow

### Secondary

- advanced Japanese learners who maintain large dictionary collections
- Anki power users interested in automation
- people who manually select dictionary definitions for cards
- users interested in personalized language-learning tooling

### Important principle

Do not optimize the message for people who do not already experience the definition-selection problem.

A small group that immediately recognizes the problem is more valuable than a large audience that does not understand why the add-on exists.

---

# 3. Core Demo to Build Later

The single highest-leverage future asset is likely a very short visual demonstration.

## 3.1 Ideal demonstration

A **15–30 second GIF/video** showing:

```text
Japanese word
      ↓
several dictionary definitions
      ↓
CompreDef evaluates candidates using learner knowledge
      ↓
a suitable definition is selected
      ↓
the selected definition appears on the card
```

The demonstration should make the value understandable without reading the repository or documentation.

## 3.2 Stronger version of the demo

Show the **same word with two different learner knowledge profiles** and demonstrate that the selected definition can change.

Conceptually:

```text
Learner A knows less Japanese
→ simpler candidate wins

Learner B knows more Japanese
→ a richer candidate can win
```

This communicates the central idea: the output is learner-aware, not merely “the first dictionary result.”

## 3.3 Example-based storytelling

The existing `人` case can be useful as a demonstration of the problem, but it should be framed carefully.

Do NOT say:

> “Yomitan gives bad definitions.”

Instead say something like:

> “Several dictionary entries can all be valid, but they are not equally useful for a particular learner. CompreDef chooses among them using the learner's existing knowledge.”

The product should criticize the **selection problem**, not imply that source dictionaries are poor quality.

---

# 4. README / GitHub Discovery Pass

Before external promotion, make the repository understandable in the first few seconds.

## 4.1 Above-the-fold goal

A new visitor should understand:

1. what CompreDef is;
2. who it is for;
3. how it differs from Yomitan;
4. what the output looks like;
5. how to install it.

The architecture should not dominate the first screen.

## 4.2 Suggested narrative order

```text
One-sentence value proposition
        ↓
Visual demo
        ↓
Very short “problem → solution” explanation
        ↓
Example
        ↓
Installation
        ↓
How it works
        ↓
Technical architecture / development details
```

## 4.3 Documentation honesty

Before release publicity, audit claims that are stronger than the evidence supports.

In particular, future documentation work should reconsider claims such as:

- “zero-disk footprint” when indexed data is persisted on disk;
- extremely specific micro-benchmark numbers unless reproducibly benchmarked;
- “0MB RAM footprint” unless literally demonstrated under a defined measurement method;
- “100% Faithful Yomitan HTML” unless candidate rendering is proven against Yomitan's native output;
- any wording that implies universal correctness of the definition ranking.

The public-facing message should be **credible and easy to verify**.

---

# 5. AnkiWeb as a Discovery Surface

AnkiWeb should be treated as a permanent product landing page, not merely an installation endpoint.

Future work could optimize:

- title;
- first paragraph;
- screenshots/GIF;
- concise explanation of the learner-aware selection problem;
- relationship to Yomitan;
- installation instructions;
- links to GitHub/documentation.

The page should answer “why would I install this?” before explaining implementation details.

---

# 6. Reddit / Community Discovery

Relevant communities include places where Japanese learners and Anki/Yomitan users already discuss workflows.

Potential communities to explore:

- `r/Anki`
- `r/LearnJapanese`
- `r/ajatt`
- other Japanese immersion / mining / Anki communities discovered later

## 6.1 Do not lead with “I made an addon”

A generic launch post is easy to ignore.

The stronger approach is **problem-first**:

```text
I kept running into this problem while using multiple Japanese dictionaries...

[show concrete example]

I built a small tool that automatically chooses a definition based on what I already know in Anki.

[show demo]
```

The discussion should be about the workflow problem first and the project second.

## 6.2 Make it complementary to Yomitan

Avoid framing the project as:

> “A replacement for Yomitan.”

Prefer:

> “A small piece that sits on top of a Yomitan/Anki workflow and solves the definition-selection step.”

The audience already invested in Yomitan is therefore the **starting market/community**, not an enemy ecosystem to displace.

## 6.3 Existing discussion threads as discovery opportunities

Later, search for discussions where people ask questions such as:

- how to use Japanese definitions instead of English;
- how to transition to monolingual dictionaries;
- how to make Yomitan definitions easier to understand;
- how to automate definition selection in Anki;
- how other learners configure Yomitan + Anki mining.

Only mention CompreDef where it genuinely answers the question. Avoid repetitive self-promotion or spam.

---

# 7. Community Content Strategy

The project can earn awareness by being useful even when the post is not explicitly promotional.

Possible future content:

## 7.1 “Monolingual transition” example

Show how learners can use multiple monolingual dictionaries while reducing the amount of manual definition selection.

## 7.2 “Why multiple dictionaries?” example

Explain that different dictionaries may provide different levels of detail, vocabulary, register, and wording, so having more dictionaries creates a **selection problem** as well as a coverage benefit.

## 7.3 Concrete before/after examples

For several Japanese words:

```text
word
→ candidate A
→ candidate B
→ candidate C
→ selected candidate
→ why the selected candidate fits this learner
```

The explanation should stay concrete and avoid exaggerated claims.

## 7.4 Workflow posts

A complete “my Japanese mining setup” post can naturally introduce CompreDef as one component of a broader Anki/Yomitan workflow.

This is likely more useful to the target audience than a generic product announcement.

---

# 8. Landing-Page / Example Gallery Idea

A very small static page or GitHub section could eventually show 3–5 examples.

Each example could display:

```text
Word
Known learner vocabulary/kanji context
Candidate definitions
Selected definition
Why it was selected
```

This would let a visitor understand the algorithm without installing anything.

Keep this tiny. It is a demonstration, not a second application.

---

# 9. Social Proof Through Real Usage

Do not optimize initially for stars, followers, or raw traffic.

Better early signals are:

- people saying “I have this exact problem”;
- users installing and trying it;
- users sharing their own examples;
- repeat usage in real mining workflows;
- bug reports that demonstrate real use;
- users requesting support for additional workflows/dictionaries.

Stars and download counts can become useful secondary signals later, but they should not define the early success criterion.

---

# 10. Lightweight Feedback Loop

After initial public exposure, collect evidence before changing the product.

Useful questions include:

- Do people understand the value proposition immediately?
- Do they understand that CompreDef is complementary to Yomitan?
- Do they understand why learner-aware ranking is useful?
- Do selected definitions feel appropriate in real cards?
- Are there recurring failure cases?
- Which dictionaries/workflows are people actually using?
- Which installation or configuration steps cause friction?

Potential future mechanism:

```text
Discovery
  ↓
Users try it
  ↓
Observe recurring problems
  ↓
Classify: documentation / bug / UX / real product gap
  ↓
Only then consider changes
```

Do not reopen stable architecture simply because one person asks for a feature.

---

# 11. Experiments to Try One at a Time

Potential experiments should be isolated so it is possible to learn what actually helps discovery.

### Experiment A — Demo-first GitHub

Improve the first screen of the README with the strongest visual demo and one-sentence value proposition.

### Experiment B — AnkiWeb optimization

Rewrite the AnkiWeb description around the problem/solution rather than implementation.

### Experiment C — Reddit problem-first post

Publish a post centered on the monolingual-transition / definition-selection problem.

### Experiment D — Workflow post

Publish a complete Yomitan + Anki workflow showing where CompreDef fits.

### Experiment E — Example gallery

Add a tiny collection of real examples demonstrating learner-aware selection.

### Experiment F — User-requested improvements only

After enough real usage, implement only changes supported by recurring user evidence.

Do not run every experiment simultaneously if doing so would make it impossible to know which change produced the effect.

---

# 12. Things NOT to Do During This Phase

Avoid:

- reworking the stable core solely for marketing reasons;
- repeatedly changing the scoring algorithm without user evidence;
- adding speculative features to increase the feature count;
- building a large web application around the project before demand exists;
- presenting CompreDef as an AI product when the core value is deterministic learner-aware selection;
- claiming that one definition is objectively “the correct” definition;
- attacking Yomitan or source dictionary projects;
- spamming multiple communities with identical promotional posts;
- optimizing for vanity metrics instead of actual usage.

---

# 13. Relationship to the Yomitan Integration / Future Technical Exploration

A separate technical exploration may eventually investigate whether CompreDef can rely more deeply on Yomitan's own structured dictionary data and native rendering rather than maintaining duplicated dictionary/rendering machinery.

This should remain **isolated from the stable core** until a proof-of-concept establishes that it can:

1. obtain structured candidate definitions;
2. preserve candidate/dictionary identity;
3. map candidate selection to exact native Yomitan rendering;
4. avoid fragile HTML parsing/splitting;
5. preserve current behavior or improve it measurably.

This is a research/proof-of-concept path, not a reason to destabilize a working release.

---

# 14. Future Strategic Question

The long-term product question is not:

> “How do we add more dictionary features?”

It is:

> **“Can CompreDef become the learner-aware selection layer between rich Japanese dictionary sources and the learner's actual Anki knowledge?”**

If real users repeatedly find that useful, that positioning can become the project's identity.

If users do not care, accept the evidence and avoid building an ecosystem around an unsupported assumption.

---

# 15. Definition of Success for This Task

This task is successful when the project has a clear, reusable discovery strategy that can be executed later without reopening the stable core.

The immediate deliverable is therefore **knowledge preservation and optional future experiments**, not code.

---

# 16. Suggested Future Execution Order

When this work is eventually activated, the conservative order is:

```text
1. Verify release/package/install experience
2. Clean README above the fold
3. Produce one excellent 15–30s demo
4. Optimize AnkiWeb presentation
5. Publish one problem-first community post
6. Observe real reactions and installations
7. Publish a workflow/example post only if useful
8. Collect recurring user evidence
9. Fix documentation/UX bugs first
10. Reconsider product changes only when evidence accumulates
```

No step should be interpreted as a commitment to continue to the next step.

---

# 17. Core Principle

**Make the project easy to discover, easy to understand, easy to try, and easy to recommend — then let actual users determine what deserves further development.**
