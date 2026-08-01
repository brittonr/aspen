let Template = import "../../profile-template.ncl" in
Template.make {
  port_classes = [
    "authority",
    "transport",
    "durable-state",
    "time",
    "scheduling",
    "membership",
    "placement",
    "supervision",
    "policy",
    "resources",
    "simulation",
    "evidence",
  ],
}
