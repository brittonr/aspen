#!/bin/bash

# Update imports from crate:: to appropriate workspace dependencies
find crates/aspen-raft/src -name "*.rs" -type f -exec sed -i \
  -e 's/use crate::api::/use aspen_core::api::/g' \
  -e 's/use crate::cluster::/use aspen_core::cluster::/g' \
  -e 's/use crate::protocol_handlers::/use aspen_core::protocol_handlers::/g' \
  -e 's/use crate::sharding::/use aspen_sharding::/g' \
  -e 's/use crate::coordination::/use aspen_coordination::/g' \
  -e 's/use crate::utils::/use aspen_core::utils::/g' \
  -e 's/use crate::raft::/use crate::/g' \
  -e 's/use super::raft::/use super::/g' {} \;

echo "Updated imports in aspen-raft crate"
