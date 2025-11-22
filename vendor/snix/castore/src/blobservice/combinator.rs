use std::sync::Arc;

use tonic::async_trait;
use tracing::instrument;

use crate::composition::{CompositionContext, ServiceBuilder};
use crate::{B3Digest, Error};

use super::{BlobReader, BlobService, BlobWriter, ChunkedReader};

/// Combinator for a BlobService, using a "near" and "far" blobservice.
/// Requests are tried in (and returned from) the near store first, only if
/// things are not present there, the far BlobService is queried.
/// In case the near blobservice doesn't have the blob, we ask the remote
/// blobservice for chunks, and try to read each of these chunks from the near
/// blobservice again, before falling back to the far one.
/// The far BlobService is never written to.
pub struct CombinedBlobService<BL, BR> {
    instance_name: String,
    near: BL,
    far: BR,
}

impl<BL, BR> Clone for CombinedBlobService<BL, BR>
where
    BL: Clone,
    BR: Clone,
{
    fn clone(&self) -> Self {
        Self {
            instance_name: self.instance_name.clone(),
            near: self.near.clone(),
            far: self.far.clone(),
        }
    }
}

#[async_trait]
impl<BL, BR> BlobService for CombinedBlobService<BL, BR>
where
    BL: AsRef<dyn BlobService> + Clone + Send + Sync + 'static,
    BR: AsRef<dyn BlobService> + Clone + Send + Sync + 'static,
{
    #[instrument(skip(self, digest), fields(blob.digest=%digest, instance_name=%self.instance_name))]
    async fn has(&self, digest: &B3Digest) -> std::io::Result<bool> {
        Ok(self.near.as_ref().has(digest).await? || self.far.as_ref().has(digest).await?)
    }

    #[instrument(skip(self, digest), fields(blob.digest=%digest, instance_name=%self.instance_name), err)]
    async fn open_read(&self, digest: &B3Digest) -> std::io::Result<Option<Box<dyn BlobReader>>> {
        if self.near.as_ref().has(digest).await? {
            // near store has the blob, so we can assume it also has all chunks.
            self.near.as_ref().open_read(digest).await
        } else {
            // near store doesn't have the blob.
            // Ask the remote one for the list of chunks,
            // and create a chunked reader that uses self.open_read() for
            // individual chunks. There's a chance we already have some chunks
            // in near, meaning we don't need to fetch them all from the far
            // BlobService.
            match self.far.as_ref().chunks(digest).await? {
                // blob doesn't exist on the near side either, nothing we can do.
                None => Ok(None),
                Some(remote_chunks) => {
                    // if there's no more granular chunks, or the far
                    // blobservice doesn't support chunks, read the blob from
                    // the far blobservice directly.
                    if remote_chunks.is_empty() {
                        return self.far.as_ref().open_read(digest).await;
                    }
                    // otherwise, a chunked reader, which will always try the
                    // near backend first.

                    let chunked_reader = ChunkedReader::from_chunks(
                        remote_chunks.into_iter().map(|chunk| {
                            (
                                chunk.digest.try_into().expect("invalid b3 digest"),
                                chunk.size,
                            )
                        }),
                        Arc::new(self.clone()) as Arc<dyn BlobService>,
                    );
                    Ok(Some(Box::new(chunked_reader)))
                }
            }
        }
    }

    #[instrument(skip_all, fields(instance_name=%self.instance_name))]
    async fn open_write(&self) -> Box<dyn BlobWriter> {
        // direct writes to the near one.
        self.near.as_ref().open_write().await
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct CombinedBlobServiceConfig {
    near: String,
    far: String,
}

impl TryFrom<url::Url> for CombinedBlobServiceConfig {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    fn try_from(_url: url::Url) -> Result<Self, Self::Error> {
        Err(Error::StorageError(
            "Instantiating a CombinedBlobService from a url is not supported".into(),
        )
        .into())
    }
}

#[async_trait]
impl ServiceBuilder for CombinedBlobServiceConfig {
    type Output = dyn BlobService;
    async fn build<'a>(
        &'a self,
        instance_name: &str,
        context: &CompositionContext,
    ) -> Result<Arc<dyn BlobService>, Box<dyn std::error::Error + Send + Sync>> {
        let (local, remote) = futures::join!(
            context.resolve(self.near.clone()),
            context.resolve(self.far.clone())
        );
        Ok(Arc::new(CombinedBlobService {
            instance_name: instance_name.to_string(),
            near: local?,
            far: remote?,
        }))
    }
}
