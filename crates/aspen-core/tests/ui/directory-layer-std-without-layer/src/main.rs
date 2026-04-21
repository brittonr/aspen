use aspen_core::DirectoryLayer;

fn main() {
    let _ = core::mem::size_of::<Option<DirectoryLayerPlaceholder>>();
}

type DirectoryLayerPlaceholder = fn() -> &'static DirectoryLayer<(), ()>;
