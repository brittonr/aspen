let Contract = import "../consumer-receipt-contract.ncl" in
(import "../../../tests/fixtures/executable-extent/molten-executable-extent-consumer-receipt.valid.json")
  | Contract.ConsumerReceipt
