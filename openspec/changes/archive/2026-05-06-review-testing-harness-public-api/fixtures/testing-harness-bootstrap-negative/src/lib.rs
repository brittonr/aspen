// This fixture intentionally depends only on reusable testing-core defaults.
// Concrete patchbay/network/madsim/bootstrap adapters must not be reachable
// unless explicitly added as adapter dependencies.

use aspen_testing_madsim as _;
use aspen_testing_network as _;
use aspen_testing_patchbay as _;

pub fn requires_adapter_dependencies() {}
