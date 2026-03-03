## ADDED Requirements

### Requirement: Service Registration

Users SHALL be able to publish named services to the cluster. A service registration maps a unique name to an iroh endpoint ID, local port, protocol, and set of tags.

#### Scenario: Publish a new service

- **GIVEN** a running Aspen cluster
- **WHEN** a user publishes a service with name `mydb`, endpoint ID `abc123...`, port `5432`, protocol `tcp`, and tags `["db", "prod"]`
- **THEN** the service SHALL be stored at KV key `/_sys/net/svc/mydb`
- **AND** the value SHALL contain the endpoint ID, port, protocol, tags, and a `published_at` timestamp

#### Scenario: Publish with duplicate name

- **GIVEN** a service named `mydb` already exists in the registry
- **WHEN** a user publishes a new service with the same name `mydb`
- **THEN** the existing registration SHALL be overwritten with the new entry
- **AND** the `published_at` timestamp SHALL be updated

#### Scenario: Service name validation

- **WHEN** a user attempts to publish a service with an invalid name (empty, longer than 253 characters, contains characters other than `[a-z0-9-.]`)
- **THEN** the operation SHALL fail with a validation error

### Requirement: Service Unpublishing

Users SHALL be able to remove a service registration by name.

#### Scenario: Unpublish an existing service

- **GIVEN** a service named `mydb` exists in the registry
- **WHEN** a user unpublishes `mydb`
- **THEN** the KV key `/_sys/net/svc/mydb` SHALL be deleted

#### Scenario: Unpublish a non-existent service

- **WHEN** a user unpublishes a service name that does not exist
- **THEN** the operation SHALL succeed silently (idempotent)

### Requirement: Service Lookup

Users and internal components SHALL be able to look up a service by name and receive its connection details.

#### Scenario: Lookup an existing service

- **GIVEN** a service named `mydb` is published with endpoint ID `abc123...` and port `5432`
- **WHEN** a lookup is performed for `mydb`
- **THEN** the result SHALL contain the endpoint ID, port, protocol, and tags

#### Scenario: Lookup a non-existent service

- **WHEN** a lookup is performed for a service name that does not exist
- **THEN** the result SHALL indicate the service was not found

### Requirement: Service Listing

Users SHALL be able to list all registered services, optionally filtered by tag.

#### Scenario: List all services

- **GIVEN** services `mydb`, `web`, and `cache` are published
- **WHEN** a user lists all services
- **THEN** the result SHALL contain all three services with their details

#### Scenario: List services by tag

- **GIVEN** `mydb` has tag `db` and `web` has tag `frontend`
- **WHEN** a user lists services filtered by tag `db`
- **THEN** the result SHALL contain only `mydb`

#### Scenario: List with no services registered

- **WHEN** a user lists services and none are registered
- **THEN** the result SHALL be an empty list

### Requirement: Node Registration

When a node publishes services, its metadata SHALL be tracked in the registry.

#### Scenario: Node metadata stored on publish

- **WHEN** a node with endpoint ID `abc123...` publishes its first service
- **THEN** a node entry SHALL be created at `/_sys/net/node/{endpoint_id_hex}`
- **AND** the entry SHALL contain the node's hostname, tags, and list of published service names

#### Scenario: Node metadata updated on subsequent publishes

- **GIVEN** a node entry exists with service `mydb`
- **WHEN** the same node publishes service `cache`
- **THEN** the node entry's service list SHALL contain both `mydb` and `cache`

### Requirement: Resource Bounds

The service registry SHALL enforce Tiger Style resource bounds.

#### Scenario: Maximum services per cluster

- **WHEN** the number of registered services reaches `MAX_NET_SERVICES` (10,000)
- **THEN** new publish operations SHALL fail with a resource exhaustion error

#### Scenario: Maximum tags per service

- **WHEN** a service is published with more than `MAX_NET_TAGS_PER_SERVICE` (32) tags
- **THEN** the operation SHALL fail with a validation error

#### Scenario: Maximum service name length

- **WHEN** a service name exceeds 253 characters
- **THEN** the operation SHALL fail with a validation error
