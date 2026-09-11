# Why lifetimes must be outcomes

The accepted design constraint is in [MODEL.md](../../MODEL.md#survival-and-death-as-outcomes). This note records its conversation basis and a bounded primary-source check on 11 September 2026. Research examples and possible extensions below are **not approved implementation rules**.

## What Pierre actually decided

The earlier discussion was recovered from the original VirtualLife orchestrator's local transcript, including messages not visible in its shortened app history. The sequence matters:

| Discussion | Evidence and meaning |
| --- | --- |
| 10 September, 10:46 Taipei time | Pierre recalled earlier experiments with distributions of lifespans and raised replenishable energy as a possibility. This described past experiments and opened a discussion; it did not select a new lifespan distribution. |
| 10 September, 10:54 | Pierre then said: "death should probably not be something determined in advance" and "Death should come from either the inner mechanisms or interaction with the environment." He requested mechanisms that could make agents vulnerable without explicitly programming death into their behaviour. |
| 10 September, 10:56 | Asked whether successful maintenance could keep an individual alive indefinitely, Pierre answered: "Keep both possibilities open for now." Vulnerability and guaranteed eventual death must therefore remain distinct. |
| 11 September | Pierre questioned powerful Repair and long-lived individuals, suggesting 50–200 ticks or less while seeking turnover across generations. He then explicitly rejected the assistant's proposed maximum lifespan and reaffirmed death as an outcome of processes. The range must not be turned into a cap or prescribed mortality curve. |
| 11 September, subsequent reserve discussion | Pierre proposed finite resources initially held by simple, largely inactive agents, required that a failing individual cannot disappear with its material, and selected recycling after wear or breakdown. He reaffirmed that everything must remain agents, without separate mechanisms, and raised shared cells as a possibility. The accepted [material-conservation direction](../../MODEL.md#finite-resources-and-conserved-material) comes from those decisions, not from the papers below; stacking remains a proposal. |

The age-limit recommendation was an assistant error, not an accepted model decision. The current code contains no lifespan field or chronological-age removal rule. Future research and summaries must preserve that distinction rather than presenting an earlier suggestion as approval.

## What the research supports

| Primary source | Relevant finding | Interpretation and limit for VirtualLife |
| --- | --- | --- |
| [Schink et al., 2019 — Death Rate of E. coli during Starvation Is Set by Maintenance Cost and Biomass Recycling](https://www.sciencedirect.com/science/article/pii/S240547121930198X) | In the studied carbon-starved populations, maintenance demand and recovery of nutrients from dead cells explain approximately exponential loss of viability. Added nutrients temporarily delay the decline. | A survival curve can follow collective supply and demand. Reproducing that curve with an independent kill lottery would discard the investigated mechanism. This study does not provide a complete account of each cell's terminal failure. |
| [Schink et al., 2024 — Survival dynamics of starving bacteria are determined by ion homeostasis that maintains plasmolysis](https://www.nature.com/articles/s41567-024-02511-2) | Active ion transport requires energy. In the studied bacteria, depolarization and swelling precede lysis; some cells recover when nutrients return before lysis. See Figures 1–2 and Extended Data Figure 4; [author-hosted full text](https://basan.med.harvard.edu/sites/g/files/omnuum9251/files/schink_et_al2024.pdf). | This gives a concrete sequence of maintenance, deterioration and possible recovery. Its mathematical model still specifies condition-dependent failure and fluctuations in ion transport. It is neither death without rules nor a recommendation to add membranes or ions here. |
| [Coelho et al., 2013 — Fission Yeast Does Not Age under Favorable Conditions, but Does So after Stress](https://www.med.upenn.edu/shorterlab/Papers/Member%20Papers/1-s2.0-S0960982213009731-main.pdf) | The studied lineages showed no progressive ageing in favourable conditions; stress-associated inheritance of large protein aggregates was associated with slower divisions and increased mortality. | Condition and inherited damage can matter more than chronological age. This is not proof of universal immortality or a universal damage law. Partitioning damage during Copy would be a new model choice. |
| [Chan — Lenia: Biology of Artificial Life](https://arxiv.org/html/1812.05433v3), sections 2.1.3, 3.5.4–3.5.5 | Local field updates sustain patterns or lead to evaporation, uncontrolled expansion, fusion and collision annihilation. | Here an individual is a pattern whose organization can disappear. VirtualLife stores separate agents with stable IDs; it currently uses an explicit integrity failure rule. Pattern dissolution would require a substantial change in representation. |

Our modelling inference is to describe what an individual must keep working, what disrupts that function, and how it can compensate. An eventual failure criterion may remain explicit in code without imposing the tick at which it must occur. Randomness can affect actual interactions or internal processes; an unexplained lethal draw is a different assumption.

## Consequences for the current model

The [current tick rules](../../MODEL.md#tick-execution-and-survival) make survival depend on integrity, upkeep, action wear and Repair. They are a small approximation of maintenance, not a detailed account of biological death. There is no material budget, and newborns receive maximum integrity. Those simplifications matter when interpreting long lives or rapid population growth.

For an isolated individual that always selects Repair, the defaults give a sustainable cycle:

```mermaid
flowchart LR
    Full[Integrity 10] -->|Upkeep costs 1| Worn[Integrity 9]
    Worn -->|Repair adds 4, capped at 10| Full
```

Nothing in that history makes maintenance harder. Imposing an age cap would hide the cause of persistence. Conversely, making every available action lose more integrity than can ever be restored would impose eventual depletion. We should investigate the balance and the source of restoration before trying to make the lifetime histogram look right.

Two candidate mechanisms follow from the research. The subsequent finite-material decision selects a direction for the first; their exact algorithms remain proposals:

- **Restoration depends on available support:** Repair uses material acquired through local processes, while wear or breakdown makes material recoverable. Individual supply, local acquisition and recycling need to be specified together. Finite world resources do not impose a personal lifespan: individuals can replenish their reserves from other holders. Treating each individual as a sealed, non-replenishable fuel tank would be a different model.
- **Condition affects function:** activity or local exposure damages the ability to maintain or restore the individual; successful compensation permits recovery. Define the causal coupling before adding another scalar called "damage". Do not silently force unavoidable decline with age.

These could eventually interact with inherited FSM parameters and mutation. They do not justify implementing a new resource layer, changing Copy inheritance, or forcing finite individual lifetimes without a model decision.

Evaluate future experiments through individual histories and cohorts: births, actual failure conditions, recoveries, lifetime distributions, and the ages of individuals still alive. A living individual's age is an unfinished observation, not its completed lifespan. A dead-only average can conceal persistent survivors. Diversity, vacancy and population balance remain results to investigate, not outcomes these mechanisms guarantee.
