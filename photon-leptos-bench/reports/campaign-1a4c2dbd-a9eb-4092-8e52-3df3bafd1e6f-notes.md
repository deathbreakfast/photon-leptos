# Campaign 1a4c2dbd-a9eb-4092-8e52-3df3bafd1e6f

## Metadata
- **Region:** us-west-2
- **AZ:** us-west-2d (all Phase 1 instance types available)
- **Staging bucket:** photon-leptos-bench-331520980087-us-west-2 _(create denied — using SSH/SCP orchestration)_
- **Orchestration:** SSH via `continuum-bench.pem` (no IAM instance profile / SSM / S3 on this account)
- **photon-leptos SHA:** 16709852647d4da1bc77cb2f8767575175eb22fe
- **photon SHA:** e836ee85bfc5c33618b91f5dcd6a44465e894165
- **Continuum pin:** ead18a741104a44bdb2c92eb495087afc4dc5dda

## Pinned AMIs (us-west-2, Ubuntu 24.04 noble gp3)
- **x86_64:** ami-0bd515973f5bcf6a0 — ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-amd64-server-20260626
- **arm64:** ami-09e85653191bf5ffe — ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-arm64-server-20260626

## Scope
4 DUT profiles: aws-t3-small, aws-t3-medium, aws-t4g-small, aws-t4g-medium
Load gens: 2× c7i.large (aws-c7i-large)

## Recovery (WSL crash 2026-07-01)

- **Recovered:** `aws-t3-medium` PLS reports from load gen `i-03d67a1d43db0c00e` (completed before crash).
- **Resumed:** `aws-t3-small` on surviving DUT `i-01d8f95423f3089d6` — PLS0/PLS1 pass; substrate bm-pl2 OOM-killed on 2 GiB RAM.
- **Blocked then fixed:** t4g DUTs received x86 tarball → `./photon-bench: Syntax error`. Native arm64 build in progress on `i-0902c6248e4f47ca3`.
- **vCPU limit:** 16 vCPU account cap; run one DUT + one load gen at a time alongside continuum fleet.

## Per-profile results

| Profile | PLS0 knee @100 | PLS0 knee @1k | PLS1 max @N=64 | Status |
|---------|----------------|---------------|----------------|--------|
| aws-t3-medium | 512 | 512 | 10000 | **done** |
| aws-t3-small | 512 | 512 | 10000 | **done** (substrate bm-pl2 failed OOM) |
| aws-t4g-small | — | — | — | **skipped** (OOM / arm64 issues) |
| aws-t4g-medium | — | — | — | **skipped** |
