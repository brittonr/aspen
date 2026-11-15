;; SPDX-License-Identifier: AGPL-3.0-or-later
;; Original import-free nonterminating workload for cancellation/fuel/deadline fixtures.
(component
  (core module $implementation
    (memory (export "memory") 1)
    (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32) i32.const 1024)
    (func (export "run") (param i32 i32) (result i32)
      (loop $spin br $spin)
      i32.const 16))
  (core instance $instance (instantiate $implementation))
  (alias core export $instance "memory" (core memory $memory))
  (alias core export $instance "cabi_realloc" (core func $realloc))
  (alias core export $instance "run" (core func $run))
  (type $bytes (list u8))
  (type $inputs (list $bytes))
  (func (export "run") (param "inputs" $inputs) (result $bytes)
    (canon lift (core func $run) (memory $memory) (realloc $realloc))))
