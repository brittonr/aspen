let Template = import "../../profile-template.ncl" in
let OversizedCpuMillis = 9000 in
Template.make {
  role_cpu_millis = OversizedCpuMillis,
}
