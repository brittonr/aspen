let Contracts = import "../contracts.ncl" in
let SourceRevision = "06cd7ca465550d2a35e0511cbbd7989e434d8f51" in
let InventoryRef = "blake3:d2d36a4685dbfcee16075e508a70592e51e6df12f86b58d039d5b961b985d367" in
let RustProfileRef = "blake3:c7dc3a3c40e1d2cc1618bac585ecbeaa5178d94b4290607018c74cf8a94aaa3c" in
let OperationGeneration = 1 in
let NodeGeneration = 1 in
let Mutations = [
  {
    mutation = "capture",
    owner = "world-commit",
    adapter_id = "world-commit-fault-adapter",
    operation_id = "blake3:51872acd0f880edab85baa934ebf5124a0656bf7e267e01a795410ae2e6d79e6",
    pre_state_ref = "blake3:7e4d93934de4dbeb0f606dc7e6a81885dce2efc12e85c0d6fb2922b6ae57bd2f",
  },
  {
    mutation = "head",
    owner = "world-head",
    adapter_id = "world-head-fault-adapter",
    operation_id = "blake3:b14119136431e10df4bfc537682a32fa3cc87f306cabacee1bf1cd4fc0e5b972",
    pre_state_ref = "blake3:9ccef7fc92c8d6c368ea78b008db2c12086d98901c82e5a19a480a1979b86d7f",
  },
  {
    mutation = "promotion",
    owner = "world-promotion",
    adapter_id = "world-promotion-fault-adapter",
    operation_id = "blake3:f91b19b636095c83735099237113d94ff61bb9c69a3e1627c627e59e10e3f94c",
    pre_state_ref = "blake3:14cef57fd9b572f9567155f2c748d7d89fd3de5a3e285e057f1f7ff65307f245",
  },
  {
    mutation = "witness",
    owner = "world-head-witness",
    adapter_id = "world-head-witness-fault-adapter",
    operation_id = "blake3:73edec2f180745e82ea7c3814059f56673e7afb5a852468ff3a92253d97a7659",
    pre_state_ref = "blake3:fb7417964c1dbc7ebcd2e55f63643231e712b2fd2eccfcb1274b1ca13d16b112",
  },
  {
    mutation = "outbox",
    owner = "world-promotion-outbox",
    adapter_id = "world-promotion-outbox-fault-adapter",
    operation_id = "blake3:ad32328a345d76bfb659dfcd2312ce23d7020f73bb14d98aed45441ae21eb226",
    pre_state_ref = "blake3:104529905535f924b7f52b397ac8b53ac8d6a0720f64ca77a3e6cb254137ad87",
  },
  {
    mutation = "replication",
    owner = "world-distribution",
    adapter_id = "world-distribution-fault-adapter",
    operation_id = "blake3:6ef40e2ddb39c86b80c91f0670aab6e403a16ba6e2b1ab94295a72bc0bb81dc0",
    pre_state_ref = "blake3:3df9ff07663d1efa948a6ae722497c48ce5064bf4c4cd4c5af3bbe4e8cf0374f",
  },
  {
    mutation = "import",
    owner = "world-replay",
    adapter_id = "world-replay-fault-adapter",
    operation_id = "blake3:a7370f3936ee474606a7c5e927330c65b38aa4a6d5e290b89d25031623ef6f33",
    pre_state_ref = "blake3:533244385a4f40e180b56b4346e5ccd32875cc79f5138add673cbf81982a39a9",
  },
  {
    mutation = "retention",
    owner = "world-retention",
    adapter_id = "world-retention-fault-adapter",
    operation_id = "blake3:42e936306b45261fa3107d9870a2f3b740582e3924e1fae9db0b2c3f0669cf86",
    pre_state_ref = "blake3:113299ba5799ef927257fa5b161c97986167eaa8536b35ab9d66238d56d897fd",
  },
  {
    mutation = "garbage-collection",
    owner = "world-garbage-collection",
    adapter_id = "world-garbage-collection-fault-adapter",
    operation_id = "blake3:83624a953c120e5fb4ec684636cc4c3780d075fd4d65f8697eda3fb3167f7732",
    pre_state_ref = "blake3:8fa92a715d1df2994a983104c9c2ffc59d6981d4c044f67d8768790d111dda2c",
  },
]
in
let Phases = [
  { phase = "uninterrupted", expected_decision = "already-complete" },
  { phase = "before-submit", expected_decision = "safe-to-retry" },
  { phase = "after-possible-submit", expected_decision = "uncertain" },
  { phase = "after-durable-write", expected_decision = "already-complete" },
  { phase = "before-response", expected_decision = "already-complete" },
  { phase = "lost-response", expected_decision = "uncertain" },
  { phase = "process-restart", expected_decision = "uncertain" },
  { phase = "recovery-read-back", expected_decision = "already-complete" },
]
in
let case = fun item phase_item =>
  {
    case_id = item.mutation ++ ":" ++ phase_item.phase,
    mutation = item.mutation,
    operation_id = item.operation_id,
    phase = phase_item.phase,
    adapter_id = item.adapter_id,
    expected_generation = OperationGeneration,
    pre_state_ref = item.pre_state_ref,
    expected_decision = if item.mutation == "witness" then "manual-review" else phase_item.expected_decision,
  }
in
let Cases =
  std.array.flatten (
    std.array.map (fun item => std.array.map (fun phase_item => case item phase_item) Phases) Mutations
  )
in
let Adapters = [
  {
    adapter_id = "world-commit-fault-adapter",
    owner = "world-commit",
    profile = "deterministic-semantic-phase-v1",
    implementation_ref = "blake3:2fbafe911f654445638cd5636eafd857238cb671869678f7c8b4fc7e2a572bf7",
    semantic_phase_map_ref = "blake3:500ccf0d7cffa3a6274b4094d6603a4191fd9329735ceabf23c9b1439c6772c7",
  },
  {
    adapter_id = "world-head-fault-adapter",
    owner = "world-head",
    profile = "deterministic-semantic-phase-v1",
    implementation_ref = "blake3:e3697d97d2ceb0ee2b4906b31986fc17f5ce757fcbc2dc6e0d9ee3a1451adaa9",
    semantic_phase_map_ref = "blake3:a1e29ef92b4bc399407af2ada40e6b5ec76cad18a265eb572fc92d1e177397a5",
  },
  {
    adapter_id = "world-promotion-fault-adapter",
    owner = "world-promotion",
    profile = "deterministic-semantic-phase-v1",
    implementation_ref = "blake3:3e7b784c65d4afe1f96907938b1caee4d42478286cfe457bc962ec7a84aa8364",
    semantic_phase_map_ref = "blake3:f486cdfe2ea1d400e5af6792bf721b2b54627993a113bf536cfa67df7d259eac",
  },
  {
    adapter_id = "world-head-witness-fault-adapter",
    owner = "world-head-witness",
    profile = "unsupported-independent-witness",
    implementation_ref = "blake3:1750d6754c6a721f6f7f6b974dfbd78d6eb38c0b7d3bd4110e1a8c4006ef1513",
    semantic_phase_map_ref = "blake3:e2d7d087979b23a497368e879e2dd916fd8a5ce9346c92dd8974e554f36b29e4",
  },
  {
    adapter_id = "world-promotion-outbox-fault-adapter",
    owner = "world-promotion-outbox",
    profile = "deterministic-semantic-phase-v1",
    implementation_ref = "blake3:93a56979f367f0c3fac3b9981a7e8cdd9b8cb0d5acc660d7da5097727d4c6fcb",
    semantic_phase_map_ref = "blake3:245f440b9a6196d9e26721c810101c6d7531d34f299a161eda4c5971ef0d8408",
  },
  {
    adapter_id = "world-distribution-fault-adapter",
    owner = "world-distribution",
    profile = "deterministic-semantic-phase-v1",
    implementation_ref = "blake3:79c3baba8bfeeb191ed1c150277f3509f6a19002765e2d82e8b252ab72a05ff8",
    semantic_phase_map_ref = "blake3:d3e38b0d8b7bfc1fd8f678f6a1b68aff13926846db77a523923674ca83a2219d",
  },
  {
    adapter_id = "world-replay-fault-adapter",
    owner = "world-replay",
    profile = "deterministic-semantic-phase-v1",
    implementation_ref = "blake3:653248188f83aab5ccbe9a6f1058ec5b3228315d628da4348fc9611faed18f5e",
    semantic_phase_map_ref = "blake3:00ddf3c66b3ef3faefcf061854b3afbda036b6e0d2ed9e8fd7c40839609e0578",
  },
  {
    adapter_id = "world-retention-fault-adapter",
    owner = "world-retention",
    profile = "deterministic-semantic-phase-v1",
    implementation_ref = "blake3:663d41ebcfcee0c082b78ba12d8b502fa73ed49be99cf0cc75e3dec57ff61e0e",
    semantic_phase_map_ref = "blake3:76bec4a3c30096fbc717b55aa4f87dd82d3cc72d5483aeb5e0856813d084634e",
  },
  {
    adapter_id = "world-garbage-collection-fault-adapter",
    owner = "world-garbage-collection",
    profile = "deterministic-semantic-phase-v1",
    implementation_ref = "blake3:54ed999e9f4dc5feb9350fbbdecececbf5137a7c8096957ea9f31d8556acd9c3",
    semantic_phase_map_ref = "blake3:72e55f04545ce42569d4a6b3bff5e6f8be32683755b10cd7d027aaf1c97e968e",
  },
]
in
let ConcurrentMutations = [
  {
    mutation = "head",
    schedule_id = "head-generation-race-v1",
    left = "blake3:b63124b9df328b94b801ad55b6e75f2818ed6c7a6406b7fa33189413f674ff49",
    right = "blake3:50eed7d65f7bdc7199775f5193a38a54c95e7da02d925c65bdeae381cbc36085",
    pre_state_ref = "blake3:f840f242c7388a8976b32f56a11876b8311ba6d9d35f0d5827ae4ef24423a93a",
  },
  {
    mutation = "promotion",
    schedule_id = "promotion-generation-race-v1",
    left = "blake3:4fde458a858dd2dc5325a0136abe450e13114f1230d200ee71503ac72f2df48c",
    right = "blake3:684cc7cea7932f558589d54993f448756b4b0b370dbf6a675f776938e6e9908a",
    pre_state_ref = "blake3:20135a7ca464b3424e3f9cc2a51263da5507facb7738e068e1d1fe76f9871c70",
  },
  {
    mutation = "witness",
    schedule_id = "witness-generation-race-v1",
    left = "blake3:023691c251d6110a1031c3c1540c1977f8b7e655741d154ff194b454a63c064f",
    right = "blake3:ac064c40ee4ee9c52989fc5210dcd4b757a5f0210a02cc2180344bf0dea61b5e",
    pre_state_ref = "blake3:531971dad7dc8bdfcf1c0fe4ed39da8d6b2197787062083b303a705578896013",
  },
  {
    mutation = "outbox",
    schedule_id = "outbox-generation-race-v1",
    left = "blake3:815f100224408de4fb7e034e3b7a2d34f6c5a8b57ab090776ea497c4630b43a2",
    right = "blake3:c3e758b22ebb8af3a9bdf27e21df37aac9b44db8b6673978c037badfeaf0950e",
    pre_state_ref = "blake3:d39474ef2f9ff58cdd6b3f39ddbaf5b2407200c73eb75ca6f364fa5b3cec4fa1",
  },
  {
    mutation = "import",
    schedule_id = "import-generation-race-v1",
    left = "blake3:5410c41190e5c2792646d1957fe2fc9df0b77d04608fa9b0015621d1ddb73868",
    right = "blake3:e6c1f7257022331cba658daee31b73c0d3ff369b3ef2ef0be11bdbaf1467a57c",
    pre_state_ref = "blake3:afd356e88db50128e4bd3e4ba5f8d5918b5b98b788637ebcc3d0662fff4f73bf",
  },
  {
    mutation = "replication",
    schedule_id = "replication-generation-race-v1",
    left = "blake3:d2da114b4c19ad592eb823f87b06cb015a0e893aef729849b1c687172a917a65",
    right = "blake3:8ce90759a3d7520c45fa4ba50996b9f680cf2cce9829ca98c749b4124452d23b",
    pre_state_ref = "blake3:729ba755f600745baca6a7bc3b35dac542c4bbde9214faace03643806cf3425a",
  },
  {
    mutation = "retention",
    schedule_id = "retention-generation-race-v1",
    left = "blake3:4085793ce4281a9cc0f25e60bedbfc0d952a78932afa973d70107e71b320fe63",
    right = "blake3:fa791858a0205b13e715b4ee01f19ef7d19599857146a3961683ee75e8f68cff",
    pre_state_ref = "blake3:1db587cb043d433e0a68a930df1792dd2f9988bcab254da6b81f13676bdd7b05",
  },
  {
    mutation = "garbage-collection",
    schedule_id = "garbage-collection-generation-race-v1",
    left = "blake3:7d548c9985b5680d83c9c948e6cdc17ef78e2f4d9b6b60d313dcba2c64b30752",
    right = "blake3:a1d78848a18101fd985284fc265aa9e3fef3330be5c4603205a622763a119b24",
    pre_state_ref = "blake3:97affefbd7c458c08d0ec553d7bbd5dc084fb5521a959af56f4b2ef908e45b1a",
  },
]
in
let step = fun item step_position operation step_interleaving step_node_id =>
  {
    position = step_position,
    operation_id = operation,
    mutation = item.mutation,
    expected_generation = OperationGeneration,
    pre_state_ref = item.pre_state_ref,
    interleaving = step_interleaving,
    node_id = step_node_id,
    node_generation = NodeGeneration,
  }
in
let pair = fun item step_position step_interleaving =>
  [
    step item step_position item.left step_interleaving "node-a",
    step item step_position item.right step_interleaving "node-b",
  ]
in
let schedule = fun item =>
  {
    schedule_id = item.schedule_id,
    mutation = item.mutation,
    winner_source = "declared-interleaving",
    wall_clock_timing = false,
    steps =
      std.array.flatten [
        pair item 0 "prepare",
        pair item 1 "current-fact-recheck",
        pair item 2 "before-linearization",
        pair item 3 "durable-read-back",
      ],
  }
in
{
  schema = "molten.world-fault-profile.v1",
  profile_name = "local-deterministic-process-restart-v1",
  source_revision = SourceRevision,
  inventory_ref = InventoryRef,
  rust_profile_ref = RustProfileRef,
  limits = {
    max_cases = 72,
    max_schedules = 8,
    max_schedule_steps = 64,
    max_adapters = 16,
    max_observations = 72,
    max_unsupported_rows = 16,
    max_restarts = 16,
  },
  adapters = Adapters,
  cases = Cases,
  schedules = std.array.map schedule ConcurrentMutations,
  unexplained_numeric_thresholds = [],
  claims_independent_witness = false,
  claims_physical_power_loss = false,
} | Contracts.Profile
