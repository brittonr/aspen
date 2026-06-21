;; Octet disabled-lint burn-down guard loop.
;;
;; This script automates the mechanical loop around semantic Rust refactors:
;;   1. Temporarily clears `dylint.toml` disabled_lints.
;;   2. Runs a no-disabled Octet probe into target/octet-burndown/probe-N.
;;   3. Restores the original `dylint.toml`.
;;   4. Optionally runs a user-provided fixer command and repeats while findings drop.
;;   5. Optionally commits and pushes only accepted iterations (`--commit-push`).
;;
;; It intentionally does not invent Rust edits. Use --fix-cmd to plug in a
;; deterministic codemod or an operator/agent step that performs exactly one
;; candidate refactor per iteration.
;;
;; Good nested-pi shape: make exactly one small refactor, do not commit/push,
;; and let this outer guard script accept, commit, and push only if validation
;; passes and the no-disabled Octet count drops.
;;
;; Example:
;;   steel scripts/octet-burndown.scm -- --max-iters 100 --commit-push \
;;     --fix-cmd 'pi -p "You are a focused Octet burn-down worker. Read .agent/napkin.md first. Current goal: reduce the no-disabled Octet finding count by exactly one or more without relaxing checks. Inspect target/octet-burndown/probe-0/summary.txt or the latest probe summary, choose one small low-risk refactor, prefer neutral same-file helper extraction in files already carrying function/file-length debt, avoid new module/path repetition, run focused validation if practical, do not commit or push, and stop after the single candidate change."' \
;;     --validate-cmd 'cargo fmt --check && cargo clippy --all-targets -- -D warnings'
;;
;; Exit behavior:
;;   - exits successfully when the baseline probe reaches 0 findings, or when
;;     a no-fix probe has reported the current count.
;;   - fails if a fixer/validator fails, a probe summary is missing, or a
;;     fixer iteration does not reduce the no-disabled finding count.
;;   - with --commit-push, commits/pushes only after an improving probe.

(require-builtin steel/process)
(require-builtin steel/core/result)

;; Keep this script runnable with the standalone Steel CLI. The pi tool prelude
;; exposes pi-args/pi-string-trim, but `steel scripts/foo.scm -- ...` does not.
(define (println text)
  (displayln text))

(define (strip-leading-separator args)
  (if (and (not (null? args)) (equal? (car args) "--"))
      (cdr args)
      args))

(define (script-arguments)
  (let ((raw (command-line)))
    (strip-leading-separator
      (if (and (not (null? raw)) (not (null? (cdr raw))))
          (cddr raw)
          '()))))

(define script-args (script-arguments))

(define (trim-string text)
  (define len (string-length text))
  (define (left index)
    (if (and (< index len) (char-whitespace? (string-ref text index)))
        (left (+ index 1))
        index))
  (define (right index)
    (if (and (> index 0) (char-whitespace? (string-ref text (- index 1))))
        (right (- index 1))
        index))
  (let* ((start (left 0))
         (end (right len)))
    (if (>= start end)
        ""
        (substring text start end))))

(define dylint-path "dylint.toml")
(define no-disabled-dylint "[octet]\ndisabled_lints = []\n")

(define (usage)
  (println "usage: steel scripts/octet-burndown.scm -- [--max-iters N] [--fix-cmd CMD] [--validate-cmd CMD] [--artifact-prefix PATH] [--commit-push]")
  (println "")
  (println "Without --fix-cmd, runs one no-disabled probe and stops after reporting findings.")
  (println "With --fix-cmd, probes baseline, then runs CMD + validate/probe until findings reach zero or stop improving.")
  (println "With --commit-push, accepted improving iterations are committed and pushed by the outer guard loop."))

(define (arg-value flag default)
  (define (loop args)
    (if (null? args)
        default
        (if (equal? (car args) flag)
            (if (null? (cdr args)) default (cadr args))
            (loop (cdr args)))))
  (loop script-args))

(define (arg-present? flag)
  (define (loop args)
    (if (null? args)
        #f
        (if (equal? (car args) flag) #t (loop (cdr args)))))
  (loop script-args))

(define (read-file path)
  (define port (open-input-file path))
  (define (loop acc)
    (let ((line (read-line port)))
      (if (eof-object? line)
          acc
          (loop (string-append acc line "\n")))))
  (define text (loop ""))
  (close-input-port port)
  text)

(define (remove-file-if-present path)
  (let* ((child (Ok->value (spawn-process (command "rm" (list "-f" "--" path)))))
         (status (Ok->value (wait child))))
    (if (not (= status 0))
        (error (string-append "failed to prepare output file " path))
        status)))

(define (write-file path text)
  ;; Steel's open-output-file overwrites from the start but does not truncate an
  ;; existing longer file in this CLI environment. Remove first so temporary
  ;; shorter configs (notably dylint.toml with disabled_lints = []) cannot leave
  ;; stale TOML tail bytes behind.
  (remove-file-if-present path)
  (define port (open-output-file path))
  (write-string text port)
  (close-output-port port))

(define (prefix? text prefix)
  (and (>= (string-length text) (string-length prefix))
       (equal? (substring text 0 (string-length prefix)) prefix)))

(define (parse-field path field)
  (define port (open-input-file path))
  (define (loop)
    (let ((line (read-line port)))
      (if (eof-object? line)
          #f
          (if (prefix? line field)
              (trim-string (substring line (string-length field)))
              (loop)))))
  (define value (loop))
  (close-input-port port)
  value)

(define (summary-findings summary-path)
  (let ((raw (parse-field summary-path "Findings:")))
    (if raw
        (string->number raw)
        #f)))

(define (summary-status summary-path)
  (let ((raw (parse-field summary-path "Status:")))
    (if raw raw "<missing>")))

(define (run-shell label script)
  (println (string-append "-- " label))
  (let* ((child (Ok->value (spawn-process (command "sh" (list "-c" script)))))
         (status (Ok->value (wait child))))
    (println (string-append "-- " label " status=" (number->string status)))
    status))

(define (must-run-shell label script)
  (let ((status (run-shell label script)))
    (if (not (= status 0))
        (error (string-append label " failed"))
        status)))

(define (probe artifact-prefix iteration original-dylint)
  (define artifact-dir (string-append artifact-prefix "-" (number->string iteration)))
  (define log-path (string-append artifact-dir ".log"))
  (define summary-path (string-append artifact-dir "/summary.txt"))
  (write-file dylint-path no-disabled-dylint)
  (let ((status
          (run-shell
            (string-append "octet probe " (number->string iteration))
            (string-append
              "mkdir -p target/octet-burndown && "
              "cargo octet check --artifact-dir " artifact-dir
              " > " log-path " 2>&1"))))
    (write-file dylint-path original-dylint)
    (if (not (= status 0))
        (begin
          (println (string-append "probe failed; restored " dylint-path "; see " log-path))
          (error "octet probe command failed"))
        (let ((findings (summary-findings summary-path)))
          (if findings
              (begin
                (println (string-append "probe " (number->string iteration)
                                        ": status=" (summary-status summary-path)
                                        " findings=" (number->string findings)
                                        " summary=" summary-path
                                        " log=" log-path))
                findings)
              (begin
                (println (string-append "missing findings in " summary-path "; restored " dylint-path))
                (error "probe summary missing Findings field")))))))

(define (run-optional label command-text)
  (if (equal? command-text "")
      0
      (run-shell label command-text)))

(define (commit-and-push previous current)
  (define message (string-append "Octet burn-down " (number->string previous) " -> " (number->string current)))
  (must-run-shell
    "commit accepted iteration"
    (string-append "git add -A && git commit -m '" message "'"))
  (must-run-shell "push accepted iteration" "git push"))

(define (accept-iteration should-commit-push previous current)
  (if should-commit-push
      (commit-and-push previous current)
      (println (string-append "Accepted improvement without commit: "
                              (number->string previous) " -> " (number->string current)))))

(define (loop-fixes iteration max-iters previous fix-cmd validate-cmd artifact-prefix original-dylint should-commit-push)
  (if (= previous 0)
      (println "Octet no-disabled probe is clean: 0 findings.")
      (if (> iteration max-iters)
          (println (string-append "Reached max iterations with findings=" (number->string previous)))
          (let ((fix-status (run-optional (string-append "fix iteration " (number->string iteration)) fix-cmd)))
            (if (not (= fix-status 0))
                (error "fix command failed")
                (let ((validate-status (run-optional (string-append "validate iteration " (number->string iteration)) validate-cmd)))
                  (if (not (= validate-status 0))
                      (error "validate command failed")
                      (let ((current (probe artifact-prefix iteration original-dylint)))
                        (if (< current previous)
                            (begin
                              (accept-iteration should-commit-push previous current)
                              (if (= current 0)
                                  (println "Octet no-disabled probe is clean: 0 findings.")
                                  (loop-fixes (+ iteration 1) max-iters current fix-cmd validate-cmd artifact-prefix original-dylint should-commit-push)))
                            (begin
                              (println (string-append "No improvement: previous=" (number->string previous)
                                                      " current=" (number->string current)))
                              (error "stopping to avoid accepting a flat/regressing refactor")))))))))))

(if (arg-present? "--help")
    (usage)
    (let* ((max-iters (string->number (arg-value "--max-iters" "1")))
           (fix-cmd (arg-value "--fix-cmd" ""))
           (validate-cmd (arg-value "--validate-cmd" ""))
           (artifact-prefix (arg-value "--artifact-prefix" "target/octet-burndown/probe"))
           (should-commit-push (arg-present? "--commit-push"))
           (original-dylint (read-file dylint-path))
           (baseline (probe artifact-prefix 0 original-dylint)))
      (if (equal? fix-cmd "")
          (println "No --fix-cmd supplied; probe-only mode complete.")
          (loop-fixes 1 max-iters baseline fix-cmd validate-cmd artifact-prefix original-dylint should-commit-push))))
