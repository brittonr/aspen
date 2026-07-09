# Plugin lifecycle FSM

Plugin lifecycle admission is a finite transition relation over current plugin state, requested lifecycle event, active manifest ref, and explicit guard facts. Receipts for install, permission, activation, hostcall, health, removal, upgrade, extension negotiation, compatibility, rollback, cleanup, and recovery are inputs to the relation, not ambient authority by possession alone.

The pure FSM core returns a decision, prior state, event, next or preserved state, selected guard refs, side-effect class, authority-closed flag, diagnostics, and canonical `plugin-lifecycle-fsm-decision-v1` evidence. Shell code may invoke callbacks, hostcalls, upgrade cutover, or removal cleanup only after this decision admits the event and names a non-`none` side-effect class.

Removal and successful cleanup close plugin-owned hostcall authority. Later hostcall or upgrade events over the same manifest preserve the removed state and deny before plugin code can run. Failed health produces a degraded prior state for use/upgrade requests unless recovery evidence is explicitly present.
