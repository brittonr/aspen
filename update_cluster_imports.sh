#!/bin/bash

# Update imports from crate:: to appropriate workspace dependencies
find crates/aspen-cluster/src -name "*.rs" -type f -exec sed -i \
  -e 's/use crate::api::/use aspen_core::api::/g' \
  -e 's/use crate::auth::/use aspen_auth::/g' \
  -e 's/use crate::blob::/use aspen_blob::/g' \
  -e 's/use crate::docs::/use iroh_docs::/g' \
  -e 's/use crate::protocol_handlers::/use aspen_transport::/g' \
  -e 's/use crate::raft::/use aspen_raft::/g' \
  -e 's/use crate::sharding::/use aspen_sharding::/g' \
  -e 's/use crate::cluster::/use crate::/g' \
  -e 's/use super::super::cluster::/use crate::/g' \
  -e 's/use super::cluster::/use super::/g' {} \;

echo "Updated imports in aspen-cluster crate"