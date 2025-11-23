# MVM-CI Worker Deployment Strategy

This directory contains comprehensive documentation for deploying workers with Flawless WASM execution.

## Quick Start

Start here: **EXECUTIVE_SUMMARY.txt** - High-level overview and recommendations

## Documents

### For Decision Makers
- **EXECUTIVE_SUMMARY.txt** - 2-page summary with timeline and recommendations
- **DEPLOYMENT_SUMMARY.txt** - Visual comparison of all 4 options

### For Implementation
- **DEPLOYMENT_STRATEGY.md** - Detailed analysis of all options (50+ pages)
- **IMPLEMENTATION_GUIDE.md** - Step-by-step setup for Option 4 (Docker Compose)

## Recommended Reading Order

1. **EXECUTIVE_SUMMARY.txt** (5 min read)
   - Understand the problem and solution
   - See the recommended timeline
   - Identify key decision points

2. **DEPLOYMENT_SUMMARY.txt** (10 min read)
   - Visual representation of each option
   - Quick comparison matrix
   - Decision tree for choosing the right option

3. **DEPLOYMENT_STRATEGY.md** (30 min read)
   - Deep dive into each option
   - Pros and cons analysis
   - Risk analysis and migration path

4. **IMPLEMENTATION_GUIDE.md** (reference)
   - Step-by-step implementation for Option 4
   - Docker Compose configuration
   - Testing checklist
   - Troubleshooting guide

## Architecture Overview

**Current State:**
- Control Plane: API server with job queue (hiqlite) + P2P (iroh)
- Worker: Binary that connects via P2P and claims jobs
- Missing: Decision on Flawless server deployment

**The Choice:**
How should workers access Flawless WASM execution when running separately?

## Four Options Analyzed

| Option | Model | When to Use | Scalability |
|--------|-------|-------------|------------|
| 1 | Each worker runs Flawless | Production (20+ workers) | Linear |
| 2 | Shared on control plane (P2P) | Tiny POCs (≤5 workers) | Poor |
| 3 | Dedicated service + load balancer | Large scale with K8s | Good |
| 4 | Shared on control plane (container network) | Short-term testing | Poor |

## Recommendation

**Phase 1 (Weeks 1-2):** Option 4 - Shared Control Plane Flawless
- Validates architecture quickly with minimal effort
- Identifies bottleneck threshold
- Clear path to production

**Phase 2 (Weeks 3-6):** Option 4 + Monitoring
- Run load tests
- Find max workers per Flawless
- Document resource requirements

**Phase 3 (Weeks 7-14):** Option 1 - Each Worker Has Its Own Flawless
- Production-ready deployment
- Complete isolation between workers
- Linear scalability

## Key Findings

1. **Architecture is 90% complete** - Worker binary and P2P framework exist
2. **Missing pieces are deployment/operations** - Not core architecture
3. **Option 4 validates with minimal effort** - Docker Compose approach
4. **Option 1 scales to production** - Each worker independent
5. **Clear decision point at week 2** - Load test determines next phase

## Timeline

```
Week 1-2:   Option 4 setup (Docker Compose)        <- START HERE
Week 3-6:   Monitoring and load testing
Week 7-14:  Option 1 implementation (MicroVM)
Week 15+:   Production scaling
```

## Next Steps

1. Read **EXECUTIVE_SUMMARY.txt**
2. Get stakeholder approval for the phased approach
3. Start **IMPLEMENTATION_GUIDE.md** implementation
4. Complete Week 1 testing within 5 working days

## Files Generated

- **Dockerfile.control-plane** - Container with mvm-ci + Flawless
- **Dockerfile.worker** - Container with worker binary
- **docker-compose.yml** - Orchestration definition
- **scripts/start-cluster.sh** - Automated cluster startup
- **src/main.rs modifications** - Export endpoint ticket
- **flake.nix modifications** - Add worker package

## Success Criteria

Week 1 Validation:
- Control plane starts successfully
- Workers connect via P2P
- Jobs execute via Flawless
- Status updates propagate correctly
- No crashes in 30-minute test run

Week 2 Metrics:
- Max workers per Flawless instance identified
- Performance characteristics documented
- Decision: Proceed with Option 1?

## Questions?

Refer to the relevant document:
- **"How do we decide?"** → EXECUTIVE_SUMMARY.txt
- **"What are all the options?"** → DEPLOYMENT_SUMMARY.txt
- **"What about option X?"** → DEPLOYMENT_STRATEGY.md
- **"How do I implement this?"** → IMPLEMENTATION_GUIDE.md
