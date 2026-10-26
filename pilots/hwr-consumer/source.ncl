let Revision = "ce9b222be993c43e9960ce7c12aed2a5130ecf76" in
{
  schema = "molten.hwr-consumer-pilot.v1",
  repository_id = "rad:z3hRCegTsS8jpJVgxYfb9psEzxHpG",
  adapter = "rad://z3hRCegTsS8jpJVgxYfb9psEzxHpG/z6MksnXbFoE8zkCkGWhHc8zuxpnEUhrJHv2KECRV4GSv9gkx",
  cargo_revision = Revision,
  maintenance_owner = "Molten Wasm component runtime maintainers (brittonr/aspen)",
  source_state = "immutable corrected-API candidate; standalone actor and denial cases executed",
  production_adoption = false,
  materialization_trust = false,
  authority = false,
  non_claims = [
    "not production runtime cutover or parity",
    "not Mantle materialization or provenance admission",
    "not actor lifecycle, persistence, replay, or canonical Preserves evidence",
    "not host isolation, whole-system safety, or release eligibility",
  ],
}
