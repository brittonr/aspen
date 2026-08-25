# Validation evidence

## Baseline

The frozen candidate passed 1,416 Nix tests, formatting, and Clippy. Pinned Octet produced warning-only evidence, but Molten's strict gate also reported stale configuration and profile hashes.

Pinned Octet status metadata:

- config: `b3:f28473b263c85516a552c0cedaa319428dd527c2fac3f8c5bfd26555dd60d73f`
- profile: `b3:939f87641be243ec8c3c7aa8e00047be314447c7ee186d1419701bb776b87ba9`

Molten recomputation before the fix:

- config: `b3:3e23f7b13b3f23dbc02e9fdccb5cf1b080688707adf3f42417b0b9d87ef014ca`
- profile: `b3:8816932189ade2d8783a5d37746f58db1acb349b4021e10d24308633ae8a28af`

Source comparison found equal fields, arguments, and raw file hashes. The serialized key order differed. Pinned Octet used lexical object-key order, while Molten preserved insertion order through unified `serde_json` features.

## Results

The canonical-order and changed-input unit tests passed. Existing Octet gate tests, the CLI warning-baseline test, formatting, and workspace Clippy with all targets and features passed.

Pinned Octet produced configuration-current warning-only evidence with 5,768 findings and no tool errors. Molten emitted deny receipt `blake3:cfa731c0daec0fb873cfcbf83057227aa1fffb825df83241644710abcb78505b` because warning-only and unreviewed critical findings remain. The receipt contains no stale configuration or profile diagnostic.

The first broad Nix run exposed one CLI fixture that independently reproduced the feature-sensitive map order. That run had 1,416 passes and one failure before the slow final test completed. The fixture now uses typed canonical payloads.

The final broad Nix rail passed 1,418 tests with no failures or skips. Its CI receipt is `blake3:0ddb0bc5c5e635d0643816532ea0f361c9161deab5a68e161f7cadd61b729dd1`, and its output is `/nix/store/m5ca98mpk1n0ldmr4q32b4vyxmg7bpni-molten-nextest`.

The fix restores metadata parity only. It does not convert Octet findings into acceptance evidence.

Strict Cairn validation and proposal, design, and task gates passed before sync. Initial receipts were proposal `9d535dfe5fbe4a58f8320b3554e7241caeb8148a78efa356b8128cd83233eb0e`, design `410a49ee2899963a64cfff123d227197a434a27146ceab221e99f9a33b93fad7`, and tasks `e5f6050a94aa1315a20389f0d2edc05f578130174acb0ced2a4156a255aef0df`.

Final pre-sync gate receipts were proposal `ea05a9e3a7e902d7993506844cd12d236c641646b379645793f3bef848426459`, design `4597e2adbcb94674ae6db57aa10e3b49306c45202a4482954222b513b2a62244`, and tasks `aa3375716c64989c1424207727ea656795001429313d1cf61714ac64ad63b4e8`. The sync dry-run plan was `b49b6aaa033148be176028ec1e66f2fd17fb8b6aad0554ce885edeab0a04794a`. Sync execution used plan `38ab69a6e7cce81e7882c594bcd9109b2f3aa719e8a2c09ab7e711f0c0047f4a` and receipt `e9b96f50dfe8b257c78d12e1a9e82f9ef51e3f4f61f536871b6bccccfeadfabf`. Strict validation passed after sync. The archive dry-run plan was `a056ebb0b45c8e202ebfb4e8fb842d230171fd3ba8bbfc26bdcf1b12e1542d7f`. Archive execution used plan `8b51a96fc176e6bb1926d8e9ab453a2815d2fe081f810f499879bfef90bfd1af` and receipt `afe2483ecfc11605ed842138fb67d72386ed350397791015d6e2b2668bc13fdd`. Strict validation passed after archive.
