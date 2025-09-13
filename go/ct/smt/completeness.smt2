
; Reference for SMT lib: 
;  - https://microsoft.github.io/z3guide/docs/theories/Sequences
;  - https://smt-lib.org/papers/smt-lib-reference-v2.0-r10.12.21.pdf

; Type definitions for modeling the EVM state ...

; Define constants for instruction set revisions
{{range $rev := .Revisions}}(define-fun {{ $rev }} () Int {{ revisionToInt $rev }})
{{end}}

; Define op-codes
{{range $op := .OpCodes}}(define-fun {{ $op }} () (_ BitVec 8) ((_ int2bv 8) {{ opToInt $op }}))
{{end}}

; Define type for the running status
(declare-datatypes () ((Status running stopped failed)))

; --- Model of EVM State ---

; Defining constant context ...
(declare-const revision Int)
(declare-const code (Seq (_ BitVec 8)))

; Define mutable context ...
(declare-datatypes () ((State (mk-state 
    (status Status)
    (stack (Seq (_ BitVec 256)))
    (pc (_ BitVec 256))
    (gas Int)
    (readOnly Bool)
    (_warmAddress (Array (_ BitVec 256) Bool))
    (_warmStorage (Array (_ BitVec 256) Bool))
    (selfAddress (_ BitVec 256))
))))
(declare-const state State)

; --- Utility Functions ---

; utility function to get the operation at a given index
(define-fun codeAt ((n (_ BitVec 256))) (_ BitVec 8) (
    ite (< (bv2int n) (seq.len code)) (seq.nth code (bv2int n)) STOP
))


; define isCode predicate - using an array
(declare-const _isCode (Array (_ BitVec 256) Bool))
(define-fun isCode ((n (_ BitVec 256))) Bool (select _isCode n))

; TODO: extend this definition to handle PUSHn op-codes
(assert (forall ((i (_ BitVec 256))) 
    (= (isCode i) (
        ite (and (>= (bv2int i) 1) (< (bv2int i) (seq.len code)))
        (and (isCode (bvsub i ((_ int2bv 256) 1))) (not (= (codeAt (bvsub i ((_ int2bv 256) 1))) PUSH1)))
        true
    ))
))

; define isData predicate as the negation of isCode
(define-fun isData ((n (_ BitVec 256))) Bool (not (isCode n)))

; define stackSize helper function
(define-fun stackSize ((s State)) Int (seq.len (stack s)))

; define param helper function
(define-fun param ((s State) (n Int)) (_ BitVec 256) (seq.nth (stack s) (- (seq.len (stack s)) (+ 1 n))))

; define warmAddress helper function
(define-fun warmAddress ((s State) (addr (_ BitVec 256))) Bool (select (_warmAddress s) addr))

; define warmStorage helper function
(define-fun warmStorage ((s State) (addr (_ BitVec 256))) Bool (select (_warmStorage s) addr))


; --- Encoding of EVM rules ---

; Number of Rules: {{len .Rules}}
{{range $rule := .Rules}}(define-fun condition_{{- escapeLiteral $rule.Name }} () Bool {{ $rule.Condition.Expression}})
{{end}}

