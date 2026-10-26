// SPDX-License-Identifier: AGPL-3.0-or-later
wit_bindgen::generate!({path:"wit",world:"batch-job"});
struct Concatenate;
impl Guest for Concatenate {
    fn run(inputs: Vec<Vec<u8>>) -> Vec<u8> {
        // Literal inputs arrive first; dependency outputs follow in sorted
        // dependency-id order. No imports, environment, files, network or clocks.
        inputs.into_iter().flatten().collect()
    }
}
export!(Concatenate);
