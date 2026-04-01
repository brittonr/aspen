# dogfood-local-connectivity

## Requirements

### CONN-1: Local node discovery

Dogfood nodes spawned on the same machine MUST discover each other without relay servers or mDNS.

### CONN-2: Client connectivity

The dogfood binary's AspenClient MUST connect to spawned nodes within 10 seconds when relay is disabled.

### CONN-3: Federation trust establishment

Alice and bob clusters MUST successfully exchange AddPeerCluster RPCs and establish bidirectional federation trust.

### CONN-4: Git push through federation

A git push to alice's forge MUST succeed, and bob MUST be able to sync the objects via federation protocol.

### CONN-5: Large repo federation sync

A repo with 100+ files in nested directories (3 levels) MUST sync completely from alice to bob. DAG integrity check on bob MUST report 0 missing objects.
