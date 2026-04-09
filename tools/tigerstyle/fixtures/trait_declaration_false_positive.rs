pub trait HotspotTrait {
    fn declaration_only(
        &self,
        alpha: u32,
        beta: u32,
        gamma: u32,
        delta: u32,
        epsilon: u32,
        zeta: u32,
        eta: u32,
        theta: u32,
    ) -> bool;
}

pub struct Worker;

impl HotspotTrait for Worker {
    fn declaration_only(
        &self,
        alpha: u32,
        beta: u32,
        gamma: u32,
        delta: u32,
        epsilon: u32,
        zeta: u32,
        eta: u32,
        theta: u32,
    ) -> bool {
        alpha + beta + gamma + delta + epsilon + zeta + eta + theta > 0
    }
}

pub fn regular_function() -> usize {
    7
}
