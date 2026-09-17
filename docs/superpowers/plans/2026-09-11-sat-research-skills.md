# SAT-Centric Research Skills Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create, validate, and deploy a suite of 8 specialized SAT-centric research skills for combinatorial optimization / HCP, synchronized with the Superpowers ecosystem and dual-deployed without making git commits.

**Architecture:** The suite is organized as a State-Aware Research Mesh across three operational clusters: Discovery & Diagnostics (sat-literature-mapping, sat-phenomenon-analysis, sat-root-cause-analysis), Theory & Falsification (sat-hypothesis-generation, sat-research-falsification, sat-experiment-design), and Synthesis & Evaluation (sat-encoding-design, sat-benchmark-analysis). It interfaces with Superpowers via explicit hand-off gates (`writing-plans`, `subagent-driven-development`, `test-driven-development`, `systematic-debugging`).

**Tech Stack:** Agentskills Markdown specification (YAML frontmatter), Python 3 test validator, Linux filesystem.

## Global Constraints

- **STRICT NO-COMMIT RULE**: Do NOT commit any files, skills, specs, or plans to git at any point during this implementation. All files remain in the workspace/plugin directory without being added to git.
- Global plugin target: `~/.gemini/config/plugins/superpowers/skills/<skill-name>/SKILL.md`
- Workspace mirror target: `/home/ubuntu/HCP/.agents/skills/<skill-name>/SKILL.md`
- Research artifact output directory: `/home/ubuntu/HCP/docs/research/`
- Every SKILL.md MUST have YAML frontmatter with `name` (matching directory) and `description` starting with `Use when...`
- Every skill MUST have explicit Hard Gates, Anti-Patterns, Step-by-Step Checklist, Artifact paths, and Superpowers Hand-off Gates.

---

### Task 1: Research Artifacts Scaffolding & Validation Test Script

**Files:**
- Create: `docs/research/{literature,phenomena,root-cause,hypotheses,falsifications,experiments,encodings,benchmarks}/.gitkeep`
- Create: `tools/verify_sat_research_skills.py`
- Test: `python3 tools/verify_sat_research_skills.py`

**Interfaces:**
- Produces: `tools/verify_sat_research_skills.py` validator testing all 8 `sat-*` skills across both global and workspace paths for valid YAML frontmatter, length limit (<= 1024 chars), required sections (Hard Gates, Checklists, Anti-patterns, Artifact paths, Superpowers hand-offs).

- [ ] **Step 1: Write the failing validator test script**
  Create `tools/verify_sat_research_skills.py` targeting the 8 `sat-*` skills.
- [ ] **Step 2: Run validator to make sure it fails (RED)**
  Execute `python3 tools/verify_sat_research_skills.py` and confirm failure due to missing skills and directories.
- [ ] **Step 3: Create directory structure in docs/research and .agents/skills**
  Create `docs/research/` subdirectories with `.gitkeep` and `.agents/skills/`.
- [ ] **Step 4: Verify test state update**
  Run `python3 tools/verify_sat_research_skills.py` to verify directories are detected and only missing skill files remain. (Remember: NO git commit).

---

### Task 2: Implement Cluster 1 — SAT Discovery & Diagnostics Skills

**Files:**
- Create: `~/.gemini/config/plugins/superpowers/skills/sat-literature-mapping/SKILL.md`
- Create: `/home/ubuntu/HCP/.agents/skills/sat-literature-mapping/SKILL.md`
- Create: `~/.gemini/config/plugins/superpowers/skills/sat-phenomenon-analysis/SKILL.md`
- Create: `/home/ubuntu/HCP/.agents/skills/sat-phenomenon-analysis/SKILL.md`
- Create: `~/.gemini/config/plugins/superpowers/skills/sat-root-cause-analysis/SKILL.md`
- Create: `/home/ubuntu/HCP/.agents/skills/sat-root-cause-analysis/SKILL.md`
- Test: `python3 tools/verify_sat_research_skills.py --cluster 1`

**Interfaces:**
- Produces: Complete SKILL.md documentation for Cluster 1 establishing the 4-tier diagnostic stack, SAT-CEGAR gap matrices, time budget decomposition, and strict heuristic prohibition gates.

- [ ] **Step 1: Implement `sat-literature-mapping` skill**
  Author comprehensive SKILL.md covering SAT-CEGAR taxonomy, theoretical CDCL bounds (Resolution lower bounds, Tseitin formulas), research gap matrix, and output templates. Mirror to `.agents/skills/`.
- [ ] **Step 2: Implement `sat-phenomenon-analysis` skill**
  Author comprehensive SKILL.md covering time budget decomposition ($T_{CDCL}$ vs auxiliary patcher), CDCL hardness cliff detection, cycle shattering analysis, multi-seed jitter rejection, and output templates. Mirror to `.agents/skills/`.
- [ ] **Step 3: Implement `sat-root-cause-analysis` skill**
  Author comprehensive SKILL.md detailing the 4-tier causal hierarchy (Graph Topology ➔ CNF Formulation & BCP ➔ CDCL Search Dynamics & LBD ➔ Engine Runtime), strict prohibition of unverified heuristic guesses, and hand-off to `systematic-debugging`. Mirror to `.agents/skills/`.
- [ ] **Step 4: Run verification test for Cluster 1**
  Run `python3 tools/verify_sat_research_skills.py --cluster 1` and confirm all Cluster 1 checks pass. (Remember: NO git commit).

---

### Task 3: Implement Cluster 2 — SAT Theory & Adversarial Falsification Skills

**Files:**
- Create: `~/.gemini/config/plugins/superpowers/skills/sat-hypothesis-generation/SKILL.md`
- Create: `/home/ubuntu/HCP/.agents/skills/sat-hypothesis-generation/SKILL.md`
- Create: `~/.gemini/config/plugins/superpowers/skills/sat-research-falsification/SKILL.md`
- Create: `/home/ubuntu/HCP/.agents/skills/sat-research-falsification/SKILL.md`
- Create: `~/.gemini/config/plugins/superpowers/skills/sat-experiment-design/SKILL.md`
- Create: `/home/ubuntu/HCP/.agents/skills/sat-experiment-design/SKILL.md`
- Test: `python3 tools/verify_sat_research_skills.py --cluster 2`

**Interfaces:**
- Produces: Complete SKILL.md documentation for Cluster 2 enforcing Chamberlin's multiple working hypotheses, adversarial trap graph mining, clause database bloat verification, and ablation matrices with CDCL internal metrics.

- [ ] **Step 1: Implement `sat-hypothesis-generation` skill**
  Author SKILL.md enforcing multiple working hypotheses ($H_1, H_2, H_3$), 3-part schema (SAT Mechanism, Quantitative Metric Prediction, Quantitative Falsification Condition), and Occam's razor. Mirror to `.agents/skills/`.
- [ ] **Step 2: Implement `sat-research-falsification` skill**
  Author SKILL.md detailing adversarial trap graph mining ($N \le 20$), clause pollution & 2-watched-literal bloat checks, seed/ordering confounder tests, and 3-test survival gate. Mirror to `.agents/skills/`.
- [ ] **Step 3: Implement `sat-experiment-design` skill**
  Author SKILL.md detailing SAT baselines (CaDiCaL, Kissat, Glucose, Takehide Soh), ablation matrices, CDCL metric suite (LBD, BCP throughput, conflict depth, PAR-2), stratification tiers, and hand-off to `writing-plans` and `dispatching-parallel-agents`. Mirror to `.agents/skills/`.
- [ ] **Step 4: Run verification test for Cluster 2**
  Run `python3 tools/verify_sat_research_skills.py --cluster 2` and confirm pass. (Remember: NO git commit).

---

### Task 4: Implement Cluster 3 — SAT Synthesis & Evaluation Skills

**Files:**
- Create: `~/.gemini/config/plugins/superpowers/skills/sat-encoding-design/SKILL.md`
- Create: `/home/ubuntu/HCP/.agents/skills/sat-encoding-design/SKILL.md`
- Create: `~/.gemini/config/plugins/superpowers/skills/sat-benchmark-analysis/SKILL.md`
- Create: `/home/ubuntu/HCP/.agents/skills/sat-benchmark-analysis/SKILL.md`
- Test: `python3 tools/verify_sat_research_skills.py --cluster 3`

**Interfaces:**
- Produces: Complete SKILL.md documentation for Cluster 3 defining formal CNF formulations, incremental assumptions, BCP strength analysis, soundness/completeness proofs, SAT Competition PAR-2 scoring, Cactus curves, and Wilcoxon significance tests.

- [ ] **Step 1: Implement `sat-encoding-design` skill**
  Author SKILL.md detailing variable semantics, degree-2 encodings, subtour elimination mechanisms (lazy cuts vs MTZ), incremental assumption interfaces, BCP propagation strength, soundness/completeness proof requirements, and hand-off to `writing-plans` + `test-driven-development`. Mirror to `.agents/skills/`.
- [ ] **Step 2: Implement `sat-benchmark-analysis` skill**
  Author SKILL.md detailing SAT Competition PAR-2 scoring, Cactus plot data generation, Wilcoxon signed-rank test ($p < 0.05$), regression root-cause breakdown, and scientific loop closure. Mirror to `.agents/skills/`.
- [ ] **Step 3: Run verification test for Cluster 3**
  Run `python3 tools/verify_sat_research_skills.py --cluster 3` and confirm pass. (Remember: NO git commit).

---

### Task 5: Implement Research Router Guide & Full System Verification

**Files:**
- Create: `docs/research/research-workflow-router.md`
- Create: `/home/ubuntu/HCP/.agents/skills/research-workflow-router.md`
- Test: `python3 tools/verify_sat_research_skills.py --all`

**Interfaces:**
- Produces: Centralized Research Router guide detailing state-machine triggers and Superpowers hand-offs, along with 100% clean verification across all 8 skills in both global and workspace directories.

- [ ] **Step 1: Author `research-workflow-router.md`**
  Document the SAT-centric research state machine, trigger conditions, transition table, and Superpowers hand-off bridges. Mirror to `.agents/skills/`.
- [ ] **Step 2: Run full verification test across all 8 skills**
  Run `python3 tools/verify_sat_research_skills.py --all` and confirm 100% compliance across both global and workspace locations.
- [ ] **Step 3: Verify clean git status (Confirming NO commits made)**
  Run `git status` to verify all new files remain untracked / uncommitted as requested by user.
