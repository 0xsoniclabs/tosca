; Experimenting with encoding of EVM rules
;
; Reference for SMT lib: 
;  - https://microsoft.github.io/z3guide/docs/theories/Sequences
;  - https://smt-lib.org/papers/smt-lib-reference-v2.0-r10.12.21.pdf


; Defining state constants ...
(declare-const code (Seq (_ BitVec 8)))

; Defining state variables ...
(declare-datatypes () ((Status Running Stopped Failed)))
(declare-datatypes () ((State (mk-state 
    (status Status)
    (pc (_ BitVec 16))
    (stack (Seq (_ BitVec 256)))
))))
(declare-const state State)

; define op-codes
(define-fun opStop () (_ BitVec 8) ((_ int2bv 8) 0))
(define-fun opAdd () (_ BitVec 8) ((_ int2bv 8) 1))
(define-fun opSub () (_ BitVec 8) ((_ int2bv 8) 2))
(define-fun opPush () (_ BitVec 8) ((_ int2bv 8) 3))
(define-fun opJump () (_ BitVec 8) ((_ int2bv 8) 4))
(define-fun opJumpDest () (_ BitVec 8) ((_ int2bv 8) 5))

; utility function to get the operation at a given index
(define-fun getOp ((n Int)) (_ BitVec 8) (
    ite (< n (seq.len code)) (seq.nth code n) opStop
))

; utility function to get the operation at the current program counter
(define-fun currentOp () (_ BitVec 8) (getOp (bv2int (pc state))))

; define isCode predicate - using an array
(declare-const _isCode (Array Int Bool))
(define-fun isCode ((n Int)) Bool (select _isCode n))

(assert (forall ((i Int)) 
    (= (isCode i) (
        ite (and (>= i 1) (< i (seq.len code)))
        (and (isCode (- i 1)) (not (= (getOp (- i 1)) opPush)))
        true
    ))
))


; utility functions to simplify the rule specification
(define-fun isRunning () Bool (= (status state) Running))
(define-fun isFailed () Bool (= (status state) Failed))

(define-fun isOp ((op (_ BitVec 8))) Bool (and (isCode (bv2int (pc state))) (= currentOp op)))

(define-fun param ((n Int)) (_ BitVec 256) (seq.nth (stack state) (- (seq.len (stack state)) (+ 1 n))))
(define-fun isValidJumpTarget () Bool (and (isCode (bv2int (param 0))) (= (getOp (bv2int (param 0))) opJumpDest)))

(define-fun setStatus ((st State) (s Status)) State ((_ update-field status) st s))
(define-fun setPc ((st State) (newPc (_ BitVec 16))) State ((_ update-field pc) st newPc))
(define-fun incPc ((st State) (n Int)) State (setPc st (bvadd (pc st) ((_ int2bv 16) n))))
(define-fun popStack ((st State) ) State ((_ update-field stack) st (seq.extract (stack st) 0 (- (seq.len (stack st)) 1))))
(define-fun pushStack ((st State) (val (_ BitVec 256))) State ((_ update-field stack) st (seq.++ (stack st) (seq.unit val))))

(define-fun fail () State (mk-state Failed #x0000 (as seq.empty (Seq (_ BitVec 256)))))

; Definition of rules for some example operators
(define-fun condition_Stop () Bool (and isRunning (isOp opStop)))
(define-fun effect_Stop () State (setStatus state Stopped))

(define-fun condition_Add_Ok () Bool (and isRunning (isOp opAdd) (>= (seq.len (stack state)) 2)))
(define-fun effect_Add_Ok () State (pushStack (popStack (popStack (incPc state 1))) (bvadd (param 0) (param 1))))

(define-fun condition_Add_Fail () Bool (and isRunning (isOp opAdd) (not (>= (seq.len (stack state)) 2))))
(define-fun effect_Add_Fail () State fail)

(define-fun condition_Sub_Ok () Bool (and isRunning (isOp opSub) (>= (seq.len (stack state)) 2)))
(define-fun effect_Sub_Ok () State (pushStack (popStack (popStack (incPc state 1))) (bvsub (param 0) (param 1))))

(define-fun condition_Sub_Fail () Bool (and isRunning (isOp opSub) (not (>= (seq.len (stack state)) 2))))
(define-fun effect_Sub_Fail () State fail)

(define-fun condition_Push () Bool (and isRunning (isOp opPush)))
(define-fun effect_Push () State (pushStack (incPc state 2) ((_ zero_extend 248) (getOp (+ (bv2int (pc state)) 1)))))

(define-fun condition_Jump_Ok () Bool (and isRunning (isOp opJump) (>= (seq.len (stack state)) 1) isValidJumpTarget))
(define-fun effect_Jump_Ok () State (setPc (popStack state) ((_ extract 15 0) (param 0))))

(define-fun condition_Jump_Fail () Bool (and isRunning (isOp opJump) (or (not (>= (seq.len (stack state)) 1)) (not isValidJumpTarget))))
(define-fun effect_Jump_Fail () State fail)

(define-fun condition_JumpDest () Bool (and isRunning (isOp opJumpDest)))
(define-fun effect_JumpDest () State (incPc state 1))


; ------------------------ Completeness Check -----------------------

(assert 
    (and
        ; general state assumptions
        (<= (seq.len code) 10)
        (<= (bv2int (pc state)) 12)

        ; complement of all rule conditions - if a state can be found that is not matched by any of those, the rules are incomplete
        (not (or
            condition_Stop
            condition_Add_Ok
            condition_Add_Fail
            condition_Sub_Ok
            condition_Sub_Fail
            condition_Push
            condition_Jump_Ok
            condition_Jump_Fail
            condition_JumpDest
            
            ; all non-running states
            (not isRunning)

            ; all undefined operations
            (and (bvugt currentOp opJumpDest))

            ; PC on data
            (and (not (isCode (bv2int (pc state)))))
        ))
    )
)
