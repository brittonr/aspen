#[cfg(test)]
mod local_roundtrip {
    use snix_castore::B3Digest;
    use snix_castore::blobservice::BlobService;
    use snix_castore::blobservice::MemoryBlobService;
    use snix_castore::directoryservice::DirectoryService;
    use snix_castore::directoryservice::RedbDirectoryService;
    use snix_castore::directoryservice::RedbDirectoryServiceConfig;
    use snix_castore::fixtures::*;
    use tokio::io::AsyncWriteExt;

    use crate::client::IrpcBlobService;
    use crate::client::IrpcDirectoryService;
    use crate::protocol::*;
    use crate::server::CastoreServer;

    /// Helper: spin up a local (in-process) castore server and return irpc clients.
    fn local_setup() -> (IrpcBlobService, IrpcDirectoryService) {
        let blob_svc = MemoryBlobService::default();
        let dir_svc = RedbDirectoryService::new_temporary("test".to_string(), RedbDirectoryServiceConfig::default())
            .expect("redb temp dir service");

        let server = CastoreServer::new(blob_svc, dir_svc);
        let tx = server.spawn();

        let client = irpc::Client::<CastoreProtocol>::local(tx);
        let blob_client = IrpcBlobService::from_client(client.clone());
        let dir_client = IrpcDirectoryService::from_client(client);
        (blob_client, dir_client)
    }

    // -----------------------------------------------------------------------
    // Blob tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn blob_roundtrip() {
        let (blob, _dir) = local_setup();

        let data = b"Hello World!";

        // Write blob
        let mut writer = blob.open_write().await;
        writer.write_all(data).await.unwrap();
        let digest = writer.close().await.unwrap();

        // Check exists
        assert!(blob.has(&digest).await.unwrap());

        // Read back
        let reader = blob.open_read(&digest).await.unwrap().expect("blob should exist");
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut tokio::io::BufReader::new(BlobReaderAdapter(reader)), &mut buf)
            .await
            .unwrap();
        assert_eq!(buf, data);
    }

    #[tokio::test]
    async fn blob_not_found() {
        let (blob, _dir) = local_setup();

        let digest = B3Digest::from(&[0u8; 32]);
        assert!(!blob.has(&digest).await.unwrap());
        assert!(blob.open_read(&digest).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn blob_empty() {
        let (blob, _dir) = local_setup();

        let data = b"";

        let mut writer = blob.open_write().await;
        writer.write_all(data).await.unwrap();
        let digest = writer.close().await.unwrap();

        assert!(blob.has(&digest).await.unwrap());

        let reader = blob.open_read(&digest).await.unwrap().expect("blob should exist");
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut tokio::io::BufReader::new(BlobReaderAdapter(reader)), &mut buf)
            .await
            .unwrap();
        assert!(buf.is_empty());
    }

    #[tokio::test]
    async fn blob_dedup() {
        let (blob, _dir) = local_setup();

        let data = b"same content";

        let mut w1 = blob.open_write().await;
        w1.write_all(data).await.unwrap();
        let d1 = w1.close().await.unwrap();

        let mut w2 = blob.open_write().await;
        w2.write_all(data).await.unwrap();
        let d2 = w2.close().await.unwrap();

        assert_eq!(d1, d2, "same content should produce same digest");
    }

    #[tokio::test]
    async fn blob_close_idempotent() {
        let (blob, _dir) = local_setup();

        let mut writer = blob.open_write().await;
        writer.write_all(b"data").await.unwrap();
        let d1 = writer.close().await.unwrap();
        let d2 = writer.close().await.unwrap();
        assert_eq!(d1, d2);
    }

    // -----------------------------------------------------------------------
    // Directory tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn dir_put_get() {
        let (_blob, dir) = local_setup();

        let directory = DIRECTORY_WITH_KEEP.clone();
        let expected_digest = directory.digest();

        let digest = dir.put(directory.clone()).await.unwrap();
        assert_eq!(digest, expected_digest);

        let got = dir.get(&digest).await.unwrap().expect("directory should exist");
        assert_eq!(got.digest(), expected_digest);
    }

    #[tokio::test]
    async fn dir_not_found() {
        let (_blob, dir) = local_setup();

        let digest = B3Digest::from(&[0u8; 32]);
        assert!(dir.get(&digest).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn dir_put_empty() {
        let (_blob, dir) = local_setup();

        let directory = DIRECTORY_A.clone(); // empty directory
        let digest = dir.put(directory.clone()).await.unwrap();
        let got = dir.get(&digest).await.unwrap().expect("should exist");
        assert_eq!(got.digest(), directory.digest());
    }

    #[tokio::test]
    async fn dir_get_recursive() {
        let (_blob, dir) = local_setup();

        // Put leaf first, then parent (leaves-to-root)
        let leaf = DIRECTORY_A.clone();
        let _leaf_digest = dir.put(leaf.clone()).await.unwrap();

        let parent = DIRECTORY_COMPLICATED.clone();
        let parent_digest = dir.put(parent.clone()).await.unwrap();

        // get_recursive from parent should stream root-to-leaves
        use futures::StreamExt;
        let dirs: Vec<_> = dir
            .get_recursive(&parent_digest)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(!dirs.is_empty());
        // First should be the parent (root)
        assert_eq!(dirs[0].digest(), parent_digest);
    }

    #[tokio::test]
    async fn dir_put_multiple() {
        let (_blob, dir) = local_setup();

        let mut putter = dir.put_multiple_start();

        // DIRECTORY_COMPLICATED references DIRECTORY_WITH_KEEP as a child.
        // DirectoryGraphBuilder expects leaves-to-root order:
        // first the leaf (DIRECTORY_WITH_KEEP), then the parent (DIRECTORY_COMPLICATED).
        let leaf = DIRECTORY_WITH_KEEP.clone();
        putter.put(leaf.clone()).await.unwrap();

        let parent = DIRECTORY_COMPLICATED.clone();
        putter.put(parent.clone()).await.unwrap();

        let root_digest = putter.close().await.unwrap();

        // Root should be the last directory put (the parent)
        assert_eq!(root_digest, parent.digest());

        // Verify both are retrievable
        assert!(dir.get(&leaf.digest()).await.unwrap().is_some());
        assert!(dir.get(&parent.digest()).await.unwrap().is_some());
    }

    // -----------------------------------------------------------------------
    // Combined blob + directory test
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn blob_and_dir_together() {
        let (blob, dir) = local_setup();

        // Write a blob
        let mut writer = blob.open_write().await;
        writer.write_all(HELLOWORLD_BLOB_CONTENTS).await.unwrap();
        let blob_digest = writer.close().await.unwrap();
        assert_eq!(blob_digest, *HELLOWORLD_BLOB_DIGEST);

        // Write a directory
        let directory = DIRECTORY_WITH_KEEP.clone();
        let dir_digest = dir.put(directory).await.unwrap();

        // Both should be retrievable
        assert!(blob.has(&blob_digest).await.unwrap());
        assert!(dir.get(&dir_digest).await.unwrap().is_some());
    }

    // -----------------------------------------------------------------------
    // Adapter for BlobReader → tokio::io::AsyncRead
    // -----------------------------------------------------------------------

    struct BlobReaderAdapter(Box<dyn snix_castore::blobservice::BlobReader>);

    impl tokio::io::AsyncRead for BlobReaderAdapter {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::pin::Pin::new(&mut *self.0).poll_read(cx, buf)
        }
    }
}
