//! Canonical Preserves constructors for manifests, chunk refs, transforms, and shared record
//! helpers.

pub(in crate::blocks) struct ChunkManifestValueInput<'a> {
    pub(in crate::blocks) object_kind: &'a str,
    pub(in crate::blocks) total_len: u64,
    pub(in crate::blocks) chunk_size: u64,
    pub(in crate::blocks) transforms: &'a crate::blocks::ChunkTransforms,
    pub(in crate::blocks) metadata_ref: &'a str,
    pub(in crate::blocks) policy_refs: &'a [String],
    pub(in crate::blocks) chunks: &'a [preserves::IOValue],
    pub(in crate::blocks) root_ref: &'a str,
    pub(in crate::blocks) evidence_refs: &'a [String],
}

pub(in crate::blocks) fn manifest_value(input: &ChunkManifestValueInput<'_>) -> preserves::IOValue {
    crate::preserves_rail::record("chunk-manifest-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::CHUNK_MANIFEST_SCHEMA),
        crate::preserves_rail::record("object-kind", vec![crate::preserves_rail::string(input.object_kind)]),
        crate::preserves_rail::record("total-len", vec![crate::preserves_rail::u64_value(input.total_len)]),
        crate::preserves_rail::record("chunker", vec![crate::preserves_rail::string(crate::blocks::FIXED_V1_CHUNKER)]),
        crate::preserves_rail::record("chunk-size", vec![crate::preserves_rail::u64_value(input.chunk_size)]),
        crate::preserves_rail::record("transforms", vec![transforms_value(input.transforms)]),
        crate::preserves_rail::record("metadata-ref", vec![crate::preserves_rail::string(input.metadata_ref)]),
        crate::preserves_rail::record("policy-refs", vec![crate::preserves_rail::sequence(
            input.policy_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("chunks", vec![crate::preserves_rail::sequence(input.chunks.to_vec())]),
        crate::preserves_rail::record("root-ref", vec![crate::preserves_rail::string(input.root_ref)]),
        crate::preserves_rail::record("evidence-refs", vec![crate::preserves_rail::sequence(
            input.evidence_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
    ])
}

pub(in crate::blocks) fn chunk_ref_value(
    chunk_ref: &str,
    length: u64,
    chunk_size: usize,
    transforms: &crate::blocks::ChunkTransforms,
) -> preserves::IOValue {
    crate::preserves_rail::record("chunk-ref-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::CHUNK_REF_SCHEMA),
        crate::preserves_rail::record("hash", vec![crate::preserves_rail::string(chunk_ref)]),
        crate::preserves_rail::record("length", vec![crate::preserves_rail::u64_value(length)]),
        crate::preserves_rail::record("domain", vec![crate::preserves_rail::string(
            crate::blocks::integrity::chunk_domain(chunk_size),
        )]),
        crate::preserves_rail::record("chunker", vec![crate::preserves_rail::string(crate::blocks::FIXED_V1_CHUNKER)]),
        crate::preserves_rail::record("transforms", vec![transforms_value(transforms)]),
        crate::preserves_rail::record("location-hints", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("local-content-key", vec![crate::preserves_rail::string(
                crate::blocks::layout::filename_for_ref(chunk_ref).unwrap_or_else(|_| "unsupported".to_string()),
            )]),
        ])]),
        crate::preserves_rail::record("evidence-refs", vec![crate::preserves_rail::sequence(Vec::new())]),
    ])
}

pub(in crate::blocks) fn transforms_value(transforms: &crate::blocks::ChunkTransforms) -> preserves::IOValue {
    crate::preserves_rail::record("transforms-v1", vec![
        crate::preserves_rail::string(crate::blocks::wire::CHUNK_TRANSFORMS_SCHEMA),
        crate::preserves_rail::record("compression", vec![crate::preserves_rail::string(&transforms.compression)]),
        crate::preserves_rail::record("encryption", vec![crate::preserves_rail::string(&transforms.encryption)]),
        crate::preserves_rail::record("ordering", vec![crate::preserves_rail::string(&transforms.ordering)]),
        crate::preserves_rail::record("confidentiality", vec![crate::preserves_rail::string(
            &transforms.confidentiality,
        )]),
        crate::preserves_rail::record("protected-commitment", vec![
            transforms
                .protected_commitment_ref
                .as_ref()
                .map(crate::preserves_rail::string)
                .unwrap_or_else(|| crate::preserves_rail::record("none", vec![])),
        ]),
    ])
}

pub(in crate::blocks) fn chunk_root_ref(chunks: &[crate::blocks::ChunkRef]) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("chunk-root-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::CHUNK_ROOT_SCHEMA),
        crate::preserves_rail::record("chunker", vec![crate::preserves_rail::string(crate::blocks::FIXED_V1_CHUNKER)]),
        crate::preserves_rail::record("chunks", vec![crate::preserves_rail::sequence(
            chunks
                .iter()
                .map(|chunk| {
                    crate::preserves_rail::record("chunk", vec![
                        crate::preserves_rail::string(&chunk.chunk_ref),
                        crate::preserves_rail::u64_value(chunk.length),
                    ])
                })
                .collect(),
        )]),
    ]))
}

pub(in crate::blocks) fn string_sequence(values: &[String]) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

pub(in crate::blocks) fn check_record(name: &str, status: &str) -> preserves::IOValue {
    crate::preserves_rail::record("check", vec![
        crate::preserves_rail::string(name),
        crate::preserves_rail::string(status),
    ])
}
