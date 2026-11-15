// SPDX-License-Identifier: AGPL-3.0-or-later
wit_bindgen::generate!({path:"wit",world:"batch-job"});
struct Render;
impl Guest for Render {
    fn run(inputs: Vec<Vec<u8>>) -> Vec<u8> {
        molten_render_pilot::execute(&inputs).expect("render assignment rejected")
    }
}
export!(Render);
