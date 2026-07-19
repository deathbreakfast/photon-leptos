# Campaign `30841be8-32e9-40bf-8547-823df3a9b5d0` notes

**Region:** us-west-2  
**Dates:** 2026-07-18 → 2026-07-19  
**Goal:** Hub A/B + key working-set (PLS5) + finish missing PLS slices + c7i ceiling + PLS9 smoke.

## Instances

| Role | Type | Notes |
|------|------|-------|
| Loadgen | c7i.large | Reused across waves |
| DUT wave 1 | t3.medium | Full slices |
| DUT wave 2 | c7i.large | PLS0/1/5 + PLS9 peer |
| DUT wave 3 (PLS9) | 2× c7i.large | Direct multi-URL (no ALB) |

Security group and AMI: account-local pins (not recorded here; set `SERVER_SG` / `AMI` when re-running).

## Sweep caveat

PLS0 N sweep is **capped at 512** in-harness (teardown of ≥1024 in-process tungstenite tasks wedged Tokio on 2-vCPU loadgens). Knees of **512** mean “passed all steps in the capped sweep,” not “failed above 512.” Hub lift above 512 needs out-of-process high-N probes.

## Wave 1 — `aws-t3-medium`

| Experiment | Mode | Result |
|------------|------|--------|
| BM-PLS0 | per_subscribe | knee **512**, pass |
| BM-PLS0-hub | broadcast_hub | knee **512**, pass (no lift vs per_subscribe within cap) |
| BM-PLS5 / PLS5-hub | both | last PASS G=**256** @ N=256; p99 ≈ 1–2 ms |
| BM-PLS1 | per_subscribe | **10k** ops/s @ N=64 |
| BM-PLS2 | per_subscribe | M=256 pass |
| BM-PLS3 | per_subscribe | refetch p99 **32** ms, pass |
| BM-PLS4 | per_subscribe | payload sweep p99 **9** ms @ last step |
| BM-PLS6 | per_subscribe | keyed vs broadcast both pass (p99 ≈ 1 ms) |
| BM-PLS7 | per_subscribe | reconnect storm pass |
| BM-PLS8 | both | **300s** soak @ N=410 (~80% of 512); p99 ≈ 40–41 ms; pass (abbreviated vs 3600s plan) |

Reports: `reports/aws-t3-medium/`.

## Wave 2 — `aws-c7i-large`

| Experiment | Mode | Result |
|------------|------|--------|
| BM-PLS0 | per_subscribe | knee **512**, pass |
| BM-PLS0-hub | broadcast_hub | knee **512**, pass |
| BM-PLS1 | both | **10k** ops/s @ N=64 |
| BM-PLS5 sweep | both | last PASS G=**256** |
| BM-PLS5 G=1 | both | pass, p99 **5** ms (full broadcast to N) |
| BM-PLS5 G=256 | both | pass, p99 **1** ms (1 recipient/key) |

Fixed CPU did **not** raise knee within the capped sweep vs t3.medium.

## Wave 3 — PLS9 (2× c7i.large, multi-URL)

- N=256 total across `http://DUT_A:8080,http://DUT_B:8080`
- p99 **3** ms, err=0, connect_fail=0 → **pass**
- Smoke only (not a full 2×512 knee proof). Direct URLs; no ALB sticky session in this run.

## Operational notes

- Do not pipe remote bench logs through `tail` over SSH (session stall).
- Prefer nohup + poll `EXIT:` in remote log files.
- `--ws-mode` with only `--server-urls` previously hit `127.0.0.1` — fixed in harness (`ensure_ws_mode` on all URLs).
- Soak abbreviated to 300s for campaign completion; both modes stable.

## Teardown

All Campaign-tagged instances terminated after PLS9.
