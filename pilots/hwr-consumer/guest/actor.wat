(component
  (core module $actor
    (memory (export "memory") 1 1)
    ;; The pilot admits one eight-byte input and performs one allocation.
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      i32.const 1024)
    (data (i32.const 80) "invalid turn")
    (func (export "turn") (param $input i32) (param $length i32) (result i32)
      local.get $length
      i32.const 8
      i32.ne
      if
        i32.const 0 i32.const 1 i32.store
        i32.const 4 i32.const 80 i32.store
        i32.const 8 i32.const 12 i32.store
      else
        ;; The result payload is a new actor state, not a constant success token.
        i32.const 64 local.get $input i32.load i32.store
        i32.const 68 local.get $input i32.load offset=4 i32.const 1 i32.add i32.store
        i32.const 0 i32.const 0 i32.store
        i32.const 4 i32.const 64 i32.store
        i32.const 8 i32.const 8 i32.store
      end
      i32.const 0))
  (core instance $actor (instantiate $actor))
  (alias core export $actor "memory" (core memory $memory))
  (alias core export $actor "realloc" (core func $realloc))
  (alias core export $actor "turn" (core func $turn))
  (type $turn-type
    (func (param "input" (list u8)) (result (result (list u8) (error string)))))
  (func $turn (type $turn-type)
    (canon lift (core func $turn) (memory $memory) (realloc $realloc)))
  (export "actor-turn" (func $turn)))
