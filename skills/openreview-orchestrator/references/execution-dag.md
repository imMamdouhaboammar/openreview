# OpenReview Orchestrator: Execution DAG

This reference details the Directed Acyclic Graph (DAG) for OpenReview tasks.

```mermaid
flowchart TD
    Start["User / PR Intent"] --> Classify{"Intent Classifier"}
    
    Classify -->|"Review Request"| ReviewNode["Mode: REVIEW"]
    Classify -->|"Fix Request"| FixNode["Mode: FIX"]
    Classify -->|"Regression / Flaky Bug"| InvestigateNode["Mode: INVESTIGATE"]
    Classify -->|"Setup / Invariant Audit"| ConfigNode["Mode: VERIFY_CONFIG"]
    
    subgraph ReviewDAG ["Review Mode Pipeline"]
        ReviewNode --> FetchDiff["Fetch Authoritative Git Diff"]
        FetchDiff --> ReadOnlySandbox["Mount Read-Only Sandbox"]
        ReadOnlySandbox --> ASTAnalysis["Analyze Logic, Security, Performance"]
        ASTAnalysis --> FormatFindings["Format 3-Tier Line-Anchored Findings"]
        FormatFindings --> ReviewOutput["Post PR Review Comments"]
    end
    
    subgraph FixDAG ["Fix Mode Pipeline"]
        FixNode --> CheckPerms{"Collaborator Permission Check"}
        CheckPerms -->|"Write / Maintain / Admin"| WritableSandbox["Spin Up Ephemeral Sandbox"]
        CheckPerms -->|"Unauthorized"| RejectFix["Reject with Diagnostic Reason"]
        WritableSandbox --> ApplyPatch["Apply Minimal Targeted Patch"]
        ApplyPatch --> RunGate{"Pre-Publish Verification Gate"}
        RunGate -->|"Tests Pass & Affirmative Proof"| CommitPush["Attributable Commit & Non-Force Push"]
        RunGate -->|"Tests Fail or Incomplete"| BlockPush["Block Push & Report Failure Trace"]
    end
    
    subgraph InvestigateDAG ["Investigate Mode (Deep-Digger)"]
        InvestigateNode --> FormulateH["Record Falsifiable Hypothesis"]
        FormulateH --> DefineOracle["Define Deterministic Test Oracle"]
        DefineOracle --> RunDdmin["Execute deep-digger-min (Rust ddmin)"]
        RunDdmin --> MinimalRepro["Extract 1-Minimal Reproducer"]
        MinimalRepro --> UpdateLedger["Update Causal Hypothesis Ledger"]
        UpdateLedger --> HandoffFix["Pass Minimal Repro to Fix Pipeline"]
    end
    
    subgraph ConfigDAG ["Config Mode"]
        ConfigNode --> RunVerifier["Execute verify_openreview.py"]
        RunVerifier --> CheckInvariants{"Evaluate 6 System Invariants"}
        CheckInvariants -->|"Pass"| ReportHealthy["Report Verified System Status"]
        CheckInvariants -->|"Fail"| ReportGaps["Report Invariant Violations & Gaps"]
    end
```

---

## State Transition Rules

1. **Review to Fix Escalation:**
   - A Review finding can transition to Fix mode only when an authorized collaborator replies with an explicit `@openreview fix` instruction.
   - Self-escalation by the agent without collaborator authorization is rejected.

2. **Fix to Investigate Fallback:**
   - If an automated fix fails the verification gate twice consecutively, the orchestrator triggers the **Circuit Breaker** and transitions to `INVESTIGATE` mode to isolate the root cause via `deep-digger-min` rather than continuing speculative edits.

3. **Termination Guarantees:**
   - Every branch of the DAG guarantees bounded execution:
     - Review: maximum 20 ReAct steps.
     - Fix: maximum 3 verification cycles before failing closed.
     - Investigate: maximum 200 oracle iterations (`max-tests`).
