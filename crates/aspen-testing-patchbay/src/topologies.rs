//! Preset topology builders for common test scenarios.
//!
//! Provides convenience constructors on `PatchbayHarness` for standard topologies.

use anyhow::Context;
use anyhow::Result;
use patchbay::RegionLink;
use patchbay::RouterPreset;

use crate::harness::PatchbayHarness;

impl PatchbayHarness {
    /// Three nodes behind a single public router (no NAT).
    ///
    /// This is the baseline topology — direct connectivity between all nodes.
    /// Use to verify patchbay infrastructure works before testing NAT scenarios.
    pub async fn three_node_public() -> Result<Self> {
        let mut harness = Self::new().await?;
        let router = harness.add_router("public", RouterPreset::Public).await?;
        for i in 0..3 {
            harness
                .add_node(&format!("node-{}", i), router, None)
                .await
                .with_context(|| format!("failed to add node-{}", i))?;
        }
        Ok(harness)
    }

    /// Three nodes, each behind its own Home NAT router.
    ///
    /// Tests iroh's NAT traversal capabilities — nodes must use relay
    /// or hole-punching to reach each other through NAT.
    pub async fn three_node_home_nat() -> Result<Self> {
        let mut harness = Self::new().await?;
        for i in 0..3 {
            let router = harness.add_router(&format!("home-{}", i), RouterPreset::Home).await?;
            harness
                .add_node(&format!("node-{}", i), router, None)
                .await
                .with_context(|| format!("failed to add node-{}", i))?;
        }
        Ok(harness)
    }

    /// Mixed NAT topology: different numbers of nodes behind Public, Home, and Corporate routers.
    ///
    /// Each node gets its own router of the specified type.
    pub async fn mixed_nat(public: u32, home: u32, corporate: u32) -> Result<Self> {
        let mut harness = Self::new().await?;
        let mut node_idx = 0u32;

        for i in 0..public {
            let router = harness.add_router(&format!("public-{}", i), RouterPreset::Public).await?;
            harness.add_node(&format!("node-{}", node_idx), router, None).await?;
            node_idx += 1;
        }

        for i in 0..home {
            let router = harness.add_router(&format!("home-{}", i), RouterPreset::Home).await?;
            harness.add_node(&format!("node-{}", node_idx), router, None).await?;
            node_idx += 1;
        }

        for i in 0..corporate {
            let router = harness.add_router(&format!("corp-{}", i), RouterPreset::Corporate).await?;
            harness.add_node(&format!("node-{}", node_idx), router, None).await?;
            node_idx += 1;
        }

        Ok(harness)
    }

    /// Two-region topology with configurable inter-region latency.
    ///
    /// Creates `eu_nodes` behind an EU router and `us_nodes` behind a US router,
    /// with `latency_ms` of simulated cross-region latency.
    pub async fn two_region(eu_nodes: u32, us_nodes: u32, latency_ms: u32) -> Result<Self> {
        let mut harness = Self::new().await?;

        // Create regions and store them
        let eu = harness.lab.add_region("eu").await?;
        let us = harness.lab.add_region("us").await?;
        harness.lab.link_regions(&eu, &us, RegionLink::good(latency_ms)).await?;
        harness.regions.insert("eu".to_string(), eu.clone());
        harness.regions.insert("us".to_string(), us.clone());

        // EU router + nodes
        let eu_router = harness.lab.add_router("dc-eu").preset(RouterPreset::Public).region(&eu).build().await?;
        let eu_router_idx = harness.routers.len();
        harness.routers.push(eu_router);

        for i in 0..eu_nodes {
            harness.add_node(&format!("eu-node-{}", i), eu_router_idx, None).await?;
        }

        // US router + nodes
        let us_router = harness.lab.add_router("dc-us").preset(RouterPreset::Public).region(&us).build().await?;
        let us_router_idx = harness.routers.len();
        harness.routers.push(us_router);

        for i in 0..us_nodes {
            harness.add_node(&format!("us-node-{}", i), us_router_idx, None).await?;
        }

        Ok(harness)
    }
}
