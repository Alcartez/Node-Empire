# Node Empire Documentation

Four kinds of document, four jobs. Nothing here should duplicate another file.

| Document | Answers |
|---|---|
| [`../README.md`](../README.md) | What is this, what works today, what is missing? |
| [`../ROADMAP.md`](../ROADMAP.md) | What is planned, in what order, and what counts as done? |
| [`MECHANICS.md`](MECHANICS.md) | Exactly what the build does, with constants and tests |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | How the code is organised and why |
| [`TESTING.md`](TESTING.md) | How to write and run tests |
| [`SETUP.md`](SETUP.md) | How to install the toolchain and build |
| [`PLAYTESTING.md`](PLAYTESTING.md) | Manual QA checklist |
| [`VISION.md`](VISION.md) | Long-term design intent (explicitly unimplemented) |
| [`RESEARCH_7K.md`](RESEARCH_7K.md) | Seven Kingdoms mechanics study and status |
| [`design/factions.md`](design/factions.md) | The six implemented courts |

## Which file should I edit?

| Change | Edit |
|---|---|
| A rule, constant, or formula | `MECHANICS.md`, plus code and a test |
| A milestone or priority | `ROADMAP.md` |
| Engine structure or tick order | `ARCHITECTURE.md` |
| A new test suite or harness helper | `TESTING.md` |
| A long-term idea with no code | `VISION.md` |
| A 7K comparison or research note | `RESEARCH_7K.md` |
| A court's multipliers | `design/factions.md` (and `types.rs`) |

## House rules

1. **One home per fact.** If it is implemented, `MECHANICS.md` owns it. If it is
   planned, `ROADMAP.md` owns it. Never both.
2. **No completion marks outside `ROADMAP.md`.**
3. **No dates or estimates.** Milestones are ordered, not scheduled.
4. **Every ✅ names a test.** If the test disappears, the claim does.
5. **No fixed test counts.** Point at `cargo test`.
6. **Design intent must be labelled.** `VISION.md` opens with a warning banner
   for a reason; do not let its ideas drift into the README.
