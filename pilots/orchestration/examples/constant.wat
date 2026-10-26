;; SPDX-License-Identifier: AGPL-3.0-or-later
(component
  (core module $implementation
    (memory (export "memory") 1)
    (global $heap (mut i32) (i32.const 1024))
    (func (export "cabi_realloc")
      (param $old i32) (param $old-size i32) (param $alignment i32) (param $new-size i32) (result i32)
      (local $pointer i32)
      global.get $heap
      local.get $alignment i32.const 1 i32.sub i32.add
      local.get $alignment i32.const 1 i32.sub i32.const -1 i32.xor i32.and
      local.tee $pointer
      local.get $new-size i32.add global.set $heap
      local.get $pointer)
    (data (i32.const 32) "ok")
    (func (export "run") (param i32 i32) (result i32)
      i32.const 16 i32.const 32 i32.store
      i32.const 20 i32.const 2 i32.store
      i32.const 16))
  (core instance $instance (instantiate $implementation))
  (alias core export $instance "memory" (core memory $memory))
  (alias core export $instance "cabi_realloc" (core func $realloc))
  (alias core export $instance "run" (core func $run))
  (type $bytes (list u8))
  (type $inputs (list $bytes))
  (func (export "run") (param "inputs" $inputs) (result $bytes)
    (canon lift (core func $run) (memory $memory) (realloc $realloc))))
