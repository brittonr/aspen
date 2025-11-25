# OpenTofu Configuration Example for Blixard State Backend
#
# This configuration shows how to use Blixard as a remote state backend
# for OpenTofu/Terraform state management.

terraform {
  # Configure the HTTP backend to use Blixard's state management
  backend "http" {
    # State endpoint - GET to retrieve, POST to update
    address = "http://localhost:8080/api/tofu/state/default"

    # Lock endpoint - POST to acquire lock
    lock_address = "http://localhost:8080/api/tofu/lock/default"

    # Unlock endpoint - POST to release lock
    unlock_address = "http://localhost:8080/api/tofu/unlock/default"

    # Lock method (default is "LOCK", but we use POST)
    lock_method = "POST"
    unlock_method = "POST"

    # Optional: API key authentication
    # Can be set via environment variable: TF_HTTP_USERNAME / TF_HTTP_PASSWORD
    # username = "api"
    # password = "your-api-key-here"
  }

  # Specify OpenTofu version requirements
  required_version = ">= 1.6.0"
}

# Example provider configuration
provider "local" {
  # This is a simple provider for testing
}

# Example resource
resource "local_file" "example" {
  content  = "Hello from OpenTofu with Blixard state backend!"
  filename = "${path.module}/hello.txt"
}

# Example output
output "file_content" {
  value = local_file.example.content
  description = "Content of the created file"
}