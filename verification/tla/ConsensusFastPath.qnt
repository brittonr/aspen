// Dev-only conformance model for the existing recovery/admission/ledger APIs.
// This is NOT a translation used by the TLC/TLA-ts parity campaign.
module ConsensusFastPath {
  pure val COMMANDS = Set("c1", "c2")
  pure val SETS = Set(Set(), Set("c1"), Set("c2"), COMMANDS)
  type ModelState = {
    phase: str, acceleration_view: int, base_view: int, last_normal_view: int,
    accepted_commands: Set[str], recovery_set: Set[str], has_marker: bool,
    applied_commands: Set[str], replied_commands: Set[str],
    commit_order: List[str], apply_order: List[str],
    last_decision: str, last_issue: str, last_conflict_free: bool,
    application_events: int, reply_events: int, duplicate_suppressed: bool
  }
  type ActionTaken = Init
    | Accept({command: str})
    | BeginViewChange({new_view: int})
    | AgreeRecoverySet({commands: Set[str]})
    | CommitRecoveryMarker({nonempty: bool})
    | ResumeNormalView({new_view: int})
    | FastCommit({command: str, ack_mode: str, promise_mode: str, conflict_free: bool})
    | Converge({command: str})
  var state: ModelState
  var action_taken: ActionTaken

  action init = all {
    state' = {
      phase: "Normal", acceleration_view: 1, base_view: 1, last_normal_view: 1,
      accepted_commands: Set(), recovery_set: Set(), has_marker: false,
      applied_commands: Set(), replied_commands: Set(), commit_order: List(), apply_order: List(),
      last_decision: "None", last_issue: "", last_conflict_free: true,
      application_events: 0, reply_events: 0, duplicate_suppressed: false
    },
    action_taken' = Init
  }

  action acceptWith(command: str): bool = all {
    state.phase == "Normal",
    not(state.accepted_commands.contains(command)),
    state' = { ...state, accepted_commands: state.accepted_commands.union(Set(command)) },
    action_taken' = Accept({command: command})
  }
  action accept = nondet command = COMMANDS.oneOf()
    acceptWith(command)

  action beginViewChangeWith(new_view: int): bool = all {
    state' = if (new_view <= state.base_view) { ...state, last_issue: "NewViewNotAhead" }
      else { ...state, phase: "Paused", base_view: new_view, last_issue: "",
        last_normal_view: if (state.phase == "Normal") state.acceleration_view else state.last_normal_view,
        recovery_set: if (state.phase == "Normal") Set() else state.recovery_set,
        has_marker: if (state.phase == "Normal") false else state.has_marker },
    action_taken' = BeginViewChange({new_view: new_view})
  }
  action beginViewChange = nondet new_view = 1.to(3).oneOf()
    beginViewChangeWith(new_view)

  action agreeRecoverySetWith(commands: Set[str]): bool = all {
    state' = if (state.phase != "Paused") { ...state, last_issue: "RecoveryNotPaused" }
      else if (not(state.accepted_commands.forall(c => commands.contains(c))) or
               not(state.recovery_set.forall(c => commands.contains(c))))
        { ...state, last_issue: "RecoverySetDropsAcceptedCommand" }
      else { ...state, recovery_set: commands, phase: "Agreed", last_issue: "" },
    action_taken' = AgreeRecoverySet({commands: commands})
  }
  action agreeRecoverySet = nondet commands = SETS.oneOf()
    agreeRecoverySetWith(commands)

  action commitRecoveryMarkerWith(nonempty: bool): bool = all {
    state' = if (state.phase != "Agreed") { ...state, last_issue: "RecoverySetNotAgreed" }
      else if (not(nonempty)) { ...state, last_issue: "EmptyMarker" }
      else { ...state, has_marker: true, phase: "MarkerCommitted", last_issue: "" },
    action_taken' = CommitRecoveryMarker({nonempty: nonempty})
  }
  action commitRecoveryMarker = nondet nonempty = Set(true, false).oneOf()
    commitRecoveryMarkerWith(nonempty)

  action resumeNormalViewWith(new_view: int): bool = all {
    state' = if (state.phase != "MarkerCommitted") { ...state, last_issue: "ResumeBeforeMarker" }
      else if (new_view != state.base_view or new_view <= state.last_normal_view)
        { ...state, last_issue: "ViewMismatch" }
      else { ...state, phase: "Normal", acceleration_view: new_view,
        accepted_commands: Set(), recovery_set: Set(), has_marker: false, last_issue: "" },
    action_taken' = ResumeNormalView({new_view: new_view})
  }
  action resumeNormalView = nondet new_view = 1.to(3).oneOf()
    resumeNormalViewWith(new_view)

  pure def expectedDecision(ack_mode: str, promise_mode: str, conflict_free: bool): str =
    if (ack_mode == "identity") "IdentityMismatch"
    else if (not(conflict_free)) "Conflict"
    else if (ack_mode == "mixed") "MixedView"
    else if (ack_mode == "short") "InsufficientAcknowledgements"
    else if (promise_mode == "mixed") "MixedView"
    else if (promise_mode == "reorder") "ReorderingProposer"
    else if (promise_mode == "missing") "MissingProposerPromise"
    // The admitted proposer is missing from a 4-of-5 same-view superquorum.
    else if (promise_mode == "unacknowledged") "MissingProposerAcknowledgement"
    else "FastCommitted"

  action fastCommitWith(command: str, ack_mode: str, promise_mode: str, conflict_free: bool): bool = {
    val decision = expectedDecision(ack_mode, promise_mode, conflict_free)
    all {
      state.phase == "Normal",
      state' = { ...state, last_decision: decision, last_conflict_free: conflict_free,
        accepted_commands: if (decision == "FastCommitted") state.accepted_commands.union(Set(command))
          else state.accepted_commands,
        commit_order: if (decision == "FastCommitted" and not(0.to(state.commit_order.length() - 1)
          .exists(i => state.commit_order.nth(i) == command)))
          state.commit_order.append(command) else state.commit_order },
      action_taken' = FastCommit({command: command, ack_mode: ack_mode,
        promise_mode: promise_mode, conflict_free: conflict_free})
    }
  }
  action fastCommit = {
    nondet command = COMMANDS.oneOf()
    nondet ack_mode = Set("same", "mixed", "short", "identity").oneOf()
    nondet promise_mode = Set("all", "missing", "mixed", "reorder", "unacknowledged").oneOf()
    nondet conflict_free = Set(true, false).oneOf()
    fastCommitWith(command, ack_mode, promise_mode, conflict_free)
  }

  // Ordering is supplied by the composition, not enforced by ApplicationLedger.
  action convergeWith(command: str): bool = all {
    0.to(state.commit_order.length() - 1).exists(i => state.commit_order.nth(i) == command),
    if (state.applied_commands.contains(command)) true
      else if (state.apply_order.length() < state.commit_order.length())
        state.commit_order.nth(state.apply_order.length()) == command else false,
    state' = { ...state,
      applied_commands: state.applied_commands.union(Set(command)),
      replied_commands: state.replied_commands.union(Set(command)),
      apply_order: if (state.applied_commands.contains(command)) state.apply_order
        else state.apply_order.append(command),
      application_events: state.application_events + (if (state.applied_commands.contains(command)) 0 else 1),
      reply_events: state.reply_events + (if (state.replied_commands.contains(command)) 0 else 1),
      duplicate_suppressed: state.applied_commands.contains(command) or state.replied_commands.contains(command) },
    action_taken' = Converge({command: command})
  }
  action converge = nondet command = COMMANDS.oneOf()
    convergeWith(command)

  action step = any { accept, beginViewChange, agreeRecoverySet, commitRecoveryMarker,
    resumeNormalView, fastCommit, converge }

  val recoverability = if (state.phase == "Agreed" or state.phase == "MarkerCommitted")
    state.accepted_commands.forall(c => state.recovery_set.contains(c)) else true
  val no_conflicting_predecessor = state.last_decision != "FastCommitted" or state.last_conflict_free
  val committed_order_agreement = 0.to(state.commit_order.length() - 1).forall(i =>
    0.to(state.commit_order.length() - 1).forall(j =>
      i == j or state.commit_order.nth(i) != state.commit_order.nth(j)))
  val execution_order_agreement = and {
    state.apply_order.length() <= state.commit_order.length(),
    0.to(state.apply_order.length() - 1).forall(i => state.apply_order.nth(i) == state.commit_order.nth(i))
  }
  val linearizable_conflict_order = execution_order_agreement and
    state.replied_commands.forall(c => state.applied_commands.contains(c))
  val at_most_once_application = state.application_events == state.applied_commands.size() and
    state.reply_events == state.replied_commands.size()
  val inv = and { recoverability, no_conflicting_predecessor, committed_order_agreement,
    execution_order_agreement, linearizable_conflict_order, at_most_once_application }

  // One state-changing input after initialization: an intended minimal guard witness.
  // The generated ITF is evidence only after the replay/mutation campaign runs.
  run mixedViewGuardTest = init.then(fastCommitWith("c1", "mixed", "all", true))
    .expect(state.last_decision == "MixedView")
  run recoveryAndDuplicateTest = init.then(acceptWith("c1"))
    .then(beginViewChangeWith(2)).then(agreeRecoverySetWith(Set()))
    .then(agreeRecoverySetWith(Set("c1"))).then(beginViewChangeWith(3))
    .then(agreeRecoverySetWith(Set("c1"))).then(commitRecoveryMarkerWith(false))
    .then(resumeNormalViewWith(3)).then(commitRecoveryMarkerWith(true))
    .then(resumeNormalViewWith(2)).then(resumeNormalViewWith(3))
    .then(fastCommitWith("c1", "same", "all", true))
    .then(convergeWith("c1")).then(convergeWith("c1"))
    .expect(inv and state.application_events == 1 and state.duplicate_suppressed)
  run emptyRecoveryTest = init.then(beginViewChangeWith(2))
    .then(agreeRecoverySetWith(Set())).then(commitRecoveryMarkerWith(true))
    .then(resumeNormalViewWith(2)).expect(inv and state.acceleration_view == 2)
  // A 4-of-5 same-view superquorum that omits the admitted active proposer (the TLA+
  // missing-promise counterexample shape), then the same command with the proposer included.
  run proposerAcknowledgementTest = init.then(fastCommitWith("c1", "same", "unacknowledged", true))
    .expect(state.last_decision == "MissingProposerAcknowledgement" and state.commit_order.length() == 0)
    .then(fastCommitWith("c1", "same", "all", true))
    .expect(inv and state.last_decision == "FastCommitted" and state.commit_order == List("c1"))
}
