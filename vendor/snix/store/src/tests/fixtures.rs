use std::{io::Cursor, sync::Arc};

use rstest::fixture;
use rstest_reuse::template;
use snix_castore::{
    blobservice::{BlobService, MemoryBlobService},
    directoryservice::{DirectoryService, MemoryDirectoryService},
    fixtures::{
        DIRECTORY_COMPLICATED, DIRECTORY_WITH_KEEP, EMPTY_BLOB_CONTENTS, EMPTY_BLOB_DIGEST,
        HELLOWORLD_BLOB_CONTENTS, HELLOWORLD_BLOB_DIGEST,
    },
};

#[fixture]
pub(crate) fn blob_service() -> Arc<dyn BlobService> {
    Arc::from(MemoryBlobService::default())
}

#[fixture]
pub(crate) async fn blob_service_with_contents() -> Arc<dyn BlobService> {
    let blob_service = Arc::from(MemoryBlobService::default());
    for (blob_contents, blob_digest) in [
        (EMPTY_BLOB_CONTENTS, &*EMPTY_BLOB_DIGEST),
        (HELLOWORLD_BLOB_CONTENTS, &*HELLOWORLD_BLOB_DIGEST),
    ] {
        // put all data into the stores.
        // insert blob into the store
        let mut writer = blob_service.open_write().await;
        tokio::io::copy(&mut Cursor::new(blob_contents), &mut writer)
            .await
            .unwrap();
        assert_eq!(blob_digest.clone(), writer.close().await.unwrap());
    }
    blob_service
}

#[fixture]
pub(crate) fn directory_service() -> Arc<dyn DirectoryService> {
    Arc::from(MemoryDirectoryService::default())
}

#[fixture]
pub(crate) async fn directory_service_with_contents() -> Arc<dyn DirectoryService> {
    let directory_service = Arc::from(MemoryDirectoryService::default());
    for directory in [&*DIRECTORY_WITH_KEEP, &*DIRECTORY_COMPLICATED] {
        directory_service.put(directory.clone()).await.unwrap();
    }
    directory_service
}

#[template]
#[rstest]
#[case::symlink(
    &crate::fixtures::CASTORE_NODE_SYMLINK,
    Ok(Ok(crate::fixtures::NAR_CONTENTS_SYMLINK.as_slice()))
)]
#[case::helloworld(
    &crate::fixtures::CASTORE_NODE_HELLOWORLD,
    Ok(Ok(crate::fixtures::NAR_CONTENTS_HELLOWORLD.as_slice()))
)]
#[case::too_big(
    &crate::fixtures::CASTORE_NODE_TOO_BIG,
    Ok(Err(io::ErrorKind::UnexpectedEof))
)]
#[case::too_small(
    &crate::fixtures::CASTORE_NODE_TOO_SMALL,
    Ok(Err(io::ErrorKind::InvalidInput))
)]
#[case::complicated(
    &crate::fixtures::CASTORE_NODE_COMPLICATED,
    Ok(Ok(crate::fixtures::NAR_CONTENTS_COMPLICATED.as_slice()))
)]
fn castore_fixtures_template(
    #[case] test_input: &Node,
    #[case] test_output: Result<Result<&[u8], io::ErrorKind>, crate::nar::RenderError>,
) {
}
