# CI agent protocol contracts
# Auto-generated Nickel contracts. Do not edit manually.

# Control messages from host to agent.
let HostMessage = [
  # Execute a command.
  {
    # Command arguments.
    args | Array String,
    # Command to execute (absolute path or PATH lookup).
    command | String,
    # Environment variables to set.
    env | {
    },
    # Unique job ID for correlation.
    id | String,
    # Execution timeout in seconds. The agent will kill the process if it exceeds this.
    timeout_secs | Number,
    type | String,
    # Working directory for execution. Must be under /workspace (virtiofs mount point).
    working_dir | String,
  },  # tag: "Execute"
  # Cancel a running job.
  {
    id | String,
    type | String,
  },  # tag: "Cancel"
  # Check if agent is alive.
  {
    type | String,
  },  # tag: "Ping"
  # Request agent shutdown.
  {
    type | String,
  },  # tag: "Shutdown"
]in

# Response messages from agent to host.
#
# Flat variants avoid nested tagged enum issues with serde.
let AgentMessage = [
  # Stdout output from execution.
  {
    data | String,
    type | String,
  },  # tag: "Stdout"
  # Stderr output from execution.
  {
    data | String,
    type | String,
  },  # tag: "Stderr"
  # Execution completed with final result.
  {
    # Execution duration in milliseconds.
    duration_ms | Number,
    # Error message if execution failed before completion.
    error | String | optional,
    # Process exit code (0 = success).
    exit_code | Number,
    # Job ID (echoed from request).
    id | String,
    # Standard error (if not streaming).
    stderr | String,
    # Standard output (if not streaming).
    stdout | String,
    type | String,
  },  # tag: "Complete"
  # Heartbeat to indicate agent is alive during long operations.
  {
    elapsed_secs | Number,
    type | String,
  },  # tag: "Heartbeat"
  # Response to ping.
  {
    type | String,
  },  # tag: "Pong"
  # Agent is ready to accept jobs.
  {
    type | String,
  },  # tag: "Ready"
  # Error response.
  {
    message | String,
    type | String,
  },  # tag: "Error"
]in

# Request from host to execute a command in the guest VM.
let ExecutionRequest = {
  # Command arguments.
  args | Array String,
  # Command to execute (absolute path or PATH lookup).
  command | String,
  # Environment variables to set.
  env | {
  },
  # Unique job ID for correlation.
  id | String,
  # Execution timeout in seconds. The agent will kill the process if it exceeds this.
  timeout_secs | Number,
  # Working directory for execution. Must be under /workspace (virtiofs mount point).
  working_dir | String,
}in

# Final result of command execution.
let ExecutionResult = {
  # Execution duration in milliseconds.
  duration_ms | Number,
  # Error message if execution failed before completion.
  error | String | optional,
  # Process exit code (0 = success).
  exit_code | Number,
  # Job ID (echoed from request).
  id | String,
  # Standard error (if not streaming).
  stderr | String,
  # Standard output (if not streaming).
  stdout | String,
}in

# Log message streamed during execution.
#
# The agent sends these incrementally as output is produced, allowing the host to stream logs to the CI system in real-time.
let LogMessage = [
  # Chunk of stdout data.
  {
    data | String,
    type | String,
  },  # tag: "Stdout"
  # Chunk of stderr data.
  {
    data | String,
    type | String,
  },  # tag: "Stderr"
  # Execution completed with final result.
  {
    data | ExecutionResult,
    type | String,
  },  # tag: "Complete"
  # Heartbeat to indicate agent is alive during long operations.
  {
    data | {
      elapsed_secs | Number,
    },
    type | String,
  },  # tag: "Heartbeat"
]in

{
  HostMessage,
  AgentMessage,
  ExecutionRequest,
  ExecutionResult,
  LogMessage,
}
