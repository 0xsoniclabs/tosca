from ast import Not
from cvc5.pythonic import *

################################################################
# Specification prover is an experimental prover for checking
# the correctness of the CT specification. The CT specification
# consists of a set of rules describing the transition from
# one state of the virtual machine to a new state. A rule is
# a triple consisting of a name, condition, and effect.
#
# A state in the VM is denoted by "s" in the set of states "State".
# A condition for rule "r" is a function cond_r: State -> Bool
# that maps a state to a boolean value. The effect for a rule
# "r" is a function effect_r: State -> State that generates
# a new state, assuming that the condition of the rule holds.
#
# For an input state s, we compute a new state s', using the
# conditional functional construct:
#
#    for all r:
#       if cond_r(s), then s'=effect_r(s)
# 
# More formally, we generate a set of states using the conditional construct:
# 
#   Out(s) = {effect_r(s)| for all r: cond_r(s))
#
# The conditional functional construct and the relation format
# of the specification lead to questions about correctness. For
# example, what if two or more conditions hold and their effect
# differ leading to different output states after the transition?
# The virtual machine would lose its deterministic behaviour.
# Another question arises whether for any virtual machine state,
# we have at least one rule to apply. 
#
# For a specification, we would like to prove two properties:
#
#   1) Determinism, i.e., forall s: |Out(s)| <= 1
#   2) Completeness, i.e., forall s: |Out(s)| > 0 
#
# If determinism and completeness hold, we have 
# 
#  forall s: |Out(s)| = 1
#
# The first property ensures that a transition produces only
# a single result state (rather than multiple ones). The second
# transition ensures that for any state, there exists a transition.
#
# The determinism property can be expressed as a first-order logic formula:
#
#   forall r<>r':
#     exists s:
#       cond_r(s) /\ cond_r'(s) => effect_r(s) = effect_r'(s)
#
# We simplify the property by abstraction. Instead of checking
# the effect, we use a unique name for different effect functions
# of rules. If we have two rules with the same effect name,
# we can assume that they have the same effect. If they have different
# names, we assume that they have a different effect, i.e., there
# exists a state such that the result states of both effect functions
# are different.
#
# This helps because the existing CT framework encodes the effects
# directly as GOLANG functions, and we cannot export them as symbolic
# functions at the moment. Another aspect of this modelling is that
# property checking is substantially simplified since the effect descriptions
# can be pretty complex.
#
# If we model effects by their name only, we have an underlying
# reachability assumption. We assume that any VM state is reachable
# (there exists a smart contract bringing the VM into this state),
# and we have the property,
#
#   for all r<>r':
#     effect_r = effect_r' =>
#       for all s:
#         effect_r(s) = effect_r'(s)
#
#   for all r<>r':
#     effect_r <> effect_r' =>
#       not exists s:
#         effect_r(s) = effect_r'(s)
#
# where effect_r and effect_r' are the names/syntax objects of the
# effect function.
#
# For using the SMT solver, we simplify the determinism property as
# follows:
#
#   for all r<>r':
#     effect_r <> effect_r' =>
#       not exists s: cond_r(s) /\ cond_r(s)
#
# For each distinct rule pair with different effect functions, we require a check
# whether there does not exist a state for which both rule conditions hold, i.e.,
# there is no state overlap between both conditions. This would imply that we
# have a state divergence and no longer a deterministic specification.
#
# The completeness check can be phrased as follows:
#
#   not exists s:
#     for all r:
#       not cond_r(s)
#
# We can rework that first-order formula to an SMT friendly formula as follows:
#
#  not exists s:
#    not (Or_r cond_r(s))
#
# This formula ensures that the virtual machine is never stuck. We can always
# transit to subsequent state.
#

# Construct VM State as a symbolic state using predicate abstraction.
# For some state description of the VM in the CT framework we find 
# abstractions that are sufficient for the SMT-solving without loss
# of precision. For example, an empty_account(x) predicate is abstracted
# with a single boolean variable since in the scope of conditions it is 
# the same whether to know whether a specific address is empty or whether
# all addresses are empty. The reason why this abstraction works is 
# because we the predicate only occurs once in a rule and does not inter-
# act with other predications (i.e., there are no dependencies).


# Virtual Machine Status
status = Int("status")
running = 0
stopped = 1
reverted = 2
failed = 3
NumStatusCodes = 4

# Predicate abstractions for cold account.
# Cold account is only used a single time in a rule, there is no need to check the address.
# It is sufficient to have a single boolean variable denoting whether the account is cold or warm.
# Cold account
def account_cold(x): # TODO: check that x is always the same variable in the context of one solver call.
    return Bool('cold_account_'+str(x))

def account_warm(x):
    return Not(account_cold(x))

# Blob check 
def hasBlobHash(x):
    return Bool('has_blob_'+str(x))

# Delegation designation
def NoDelegationDesignation(x):
    return Not(Bool('deleg_desig_'+str(x)))

def ColdDelegationDesignation(x):
    return And(Not(NoDelegationDesignation(x)), Bool('cold_deleg_'+str(x)))

def WarmDelegationDesignation(x):
    return And(Not(NoDelegationDesignation(x)), Not(Bool('cold_deleg_'+str(x))))

# Account empty check
def account_empty(x):
    return Bool('account_empty_'+str(x))

# Function abstraction for balance with integer variable for the result.
# The balance function is only used a single time in a rule, there is no need to check the address.}

balance_dic = {}
def balance(x):
    if str(x) in balance_dic:
        return balance_dic[str(x)]
    else:
        var = Int('balance_'+str(x))
        balance_dic[str(x)] = var
        return var

def tranStorageNonZero(x):
    return Bool('tran_storage_'+str(x))

def tranStorageToZero(x):
    return Not(tranStorageNonZero(x))

# Self address (might not be used)
self = Int('self')

# Has self-destructed state
hasSelfDestructed = Bool('hasSelfDestructed')

def inRange256FromCurrentBlock(x):
    return Bool('inRange_'+str(x))

storage_cold_dic = {}
def storage_cold(x):
    if str(x) in storage_cold_dic:
        return storage_cold_dic[str(x)]
    else:
        var = Bool('storage_cold_'+str(x))
        storage_cold_dic[str(x)] = var
        return var

# Storage configuration abstraction
StorageAssigned = 0
StorageAdded = 1
StorageDeleted = 2
StorageModified = 3
StorageDeletedAdded = 4
StorageModifiedDeleted = 5
StorageDeletedRestored = 6
StorageAddedDeleted = 7
StorageModifiedRestored = 8
numStorageStatus = 9

storage_conf_dic = {}
def storageConf(x,y,z):
    if (y,z) in storage_conf_dic:
        return storage_conf_dic[(y,z)] == x
    else:
        var = Int('storageConf_'+str(y)+'_'+str(z))
        storage_conf_dic[(y,z)] = var
        return  storage_conf_dic[(y,z)] == x

# ReadOnly Flag for static call execution
readOnly = Bool('readOnly')

# Gas
gas = Int("gas")

# Stack variables 
stackSize = Int("stackSize")
stack = Array("stack", IntSort(), IntSort())

# Variable abstraction for stack parameters
def param(x):
    sdata = Int("sdata_"+str(x))   # param variable needs to be unique (e.g. op_pc, etc.)
    sdata = Select(stack, stackSize - x - 1)
    return sdata

# Revision
revision = Int("revision")
Istanbul = 0
Berlin = 1
London = 2
Paris = 3
Shanghai = 4
Cancun = 5
Prague = 6
numRevisions = 7


# Program Counter
pc = Int("pc")
MaxCodeLen = 16384 + 8192

# Code Block
code_block = Array("code_block", IntSort(), IntSort())

# EVM Instructions
STOP            = 0x00
ADD             = 0x01
MUL             = 0x02
SUB             = 0x03
DIV             = 0x04
SDIV            = 0x05
MOD             = 0x06
SMOD            = 0x07
ADDMOD          = 0x08
MULMOD          = 0x09
EXP             = 0x0A
SIGNEXTEND      = 0x0B
LT              = 0x10
GT              = 0x11
SLT             = 0x12
SGT             = 0x13
EQ              = 0x14
ISZERO          = 0x15
AND             = 0x16
OR              = 0x17
XOR             = 0x18
NOT             = 0x19
BYTE            = 0x1A
SHL             = 0x1B
SHR             = 0x1C
SAR             = 0x1D
SHA3            = 0x20
ADDRESS         = 0x30
BALANCE         = 0x31
ORIGIN          = 0x32
CALLER          = 0x33
CALLVALUE       = 0x34
CALLDATALOAD    = 0x35
CALLDATASIZE    = 0x36
CALLDATACOPY    = 0x37
CODESIZE        = 0x38
CODECOPY        = 0x39
GASPRICE        = 0x3A
EXTCODESIZE     = 0x3B
EXTCODECOPY     = 0x3C
RETURNDATASIZE  = 0x3D
RETURNDATACOPY  = 0x3E
EXTCODEHASH     = 0x3F
BLOCKHASH       = 0x40
COINBASE        = 0x41
TIMESTAMP       = 0x42
NUMBER          = 0x43
PREVRANDAO      = 0x44
GASLIMIT        = 0x45
CHAINID         = 0x46
SELFBALANCE     = 0x47
BASEFEE         = 0x48
BLOBHASH        = 0x49
BLOBBASEFEE     = 0x4A
POP             = 0x50
MLOAD           = 0x51
MSTORE          = 0x52
MSTORE8         = 0x53
SLOAD           = 0x54
SSTORE          = 0x55
JUMP            = 0x56
JUMPI           = 0x57
PC              = 0x58
MSIZE           = 0x59
GAS             = 0x5A
JUMPDEST        = 0x5B
TLOAD           = 0x5C
TSTORE          = 0x5D
PUSH0           = 0x5F
MCOPY           = 0x5E
PUSH1           = 0x60
PUSH2           = 0x61
PUSH3           = 0x62
PUSH4           = 0x63
PUSH5           = 0x64
PUSH6           = 0x65
PUSH7           = 0x66
PUSH8           = 0x67
PUSH9           = 0x68
PUSH10          = 0x69
PUSH11          = 0x6A
PUSH12          = 0x6B
PUSH13          = 0x6C
PUSH14          = 0x6D
PUSH15          = 0x6E
PUSH16          = 0x6F
PUSH17          = 0x70
PUSH18          = 0x71
PUSH19          = 0x72
PUSH20          = 0x73
PUSH21          = 0x74
PUSH22          = 0x75
PUSH23          = 0x76
PUSH24          = 0x77
PUSH25          = 0x78
PUSH26          = 0x79
PUSH27          = 0x7A
PUSH28          = 0x7B
PUSH29          = 0x7C
PUSH30          = 0x7D
PUSH31          = 0x7E
PUSH32          = 0x7F
DUP1            = 0x80
DUP2            = 0x81
DUP3            = 0x82
DUP4            = 0x83
DUP5            = 0x84
DUP6            = 0x85
DUP7            = 0x86
DUP8            = 0x87
DUP9            = 0x88
DUP10           = 0x89
DUP11           = 0x8A
DUP12           = 0x8B
DUP13           = 0x8C
DUP14           = 0x8D
DUP15           = 0x8E
DUP16           = 0x8F
SWAP1           = 0x90
SWAP2           = 0x91
SWAP3           = 0x92
SWAP4           = 0x93
SWAP5           = 0x94
SWAP6           = 0x95
SWAP7           = 0x96
SWAP8           = 0x97
SWAP9           = 0x98
SWAP10          = 0x99
SWAP11          = 0x9A
SWAP12          = 0x9B
SWAP13          = 0x9C
SWAP14          = 0x9D
SWAP15          = 0x9E
SWAP16          = 0x9F
LOG0            = 0xA0
LOG1            = 0xA1
LOG2            = 0xA2
LOG3            = 0xA3
LOG4            = 0xA4
CREATE          = 0xF0
CALL            = 0xF1
CALLCODE        = 0xF2
RETURN          = 0xF3
DELEGATECALL    = 0xF4
CREATE2         = 0xF5
STATICCALL      = 0xFA
REVERT          = 0xFD
INVALID         = 0xFE
SELFDESTRUCT    = 0xFF

# Fetch operation on code block
def code(x):
    op = Int("op_"+str(x))   # opcode variable needs to be unique (e.g. op_pc, etc.)
    op = Select(code_block, x)
    return op

# TODO: A predicate abstraction for code is inadequate because for JUMP
# instructions we have two isCode/isData predicates per rule. Instead of
# a predicate abstraction, we need a variable abstraction. I.e., we need
# a boolean variable is_code_pc for code(pc) and a boolean variable 
# is_code_param_0 for code(param(0)).
# is_code_dict = {}
# def isCode(x):
#     if str(x) in is_code_dict:
#         return is_code_dict[str(x)]
#     else:
#         var = Bool('is_code_'+str(x))
#         is_code_dict[str(x)] = var
#        return var

def isCode(x):
    return Bool("is_code_"+str(x))   # is_code variable needs to be unique (e.g. is_code_pc, etc.) 


def isData(x):
    return Not(isCode(x))

# op for illegal operations
def op(x):
    return x

# build solver that adds state constraints for constructing valid states
def vm_state_solver():
    # Initialize a solver
    s = Solver()

    # add state constraints to produce overlapping state
    # examples over valid states only.

    # bound revision variable
    s.add(revision >= 0, revision < numRevisions)

    # bound status
    s.add(status >= 0, status < NumStatusCodes)

    # bound program counter
    s.add(pc >= 0, pc < MaxCodeLen)

    # bound gas counter
    s.add(gas >= 0)

    # bound balance
    for x in balance_dic:
        s.add(balance_dic[x] >= 0)

    # bound stack size
    s.add(stackSize >= 0, stackSize <= 1024)

    # bound storage status
    for conf in storage_conf_dic:
        s.add(storage_conf_dic[conf] >= 0, storage_conf_dic[conf] < numStorageStatus)

    # storage configuration, cold access
    for cold in storage_cold_dic:
        for conf in storage_conf_dic:
            s.add(Implies(storage_conf_dic[conf] == StorageAssigned, Not(storage_cold_dic[cold])))
            s.add(Implies(storage_conf_dic[conf] == StorageAddedDeleted, Not(storage_cold_dic[cold])))
            s.add(Implies(storage_conf_dic[conf] == StorageDeletedRestored, Not(storage_cold_dic[cold])))
            s.add(Implies(storage_conf_dic[conf] == StorageDeletedAdded, Not(storage_cold_dic[cold])))
            s.add(Implies(storage_conf_dic[conf] == StorageModifiedDeleted, Not(storage_cold_dic[cold])))
            s.add(Implies(storage_conf_dic[conf] == StorageModifiedRestored, Not(storage_cold_dic[cold])))

    # bound op-code to byte range
    s.add(code(pc) >= 0, code(pc) <= 255)

    return s


def is_overlapping(c_i, c_j):
    # build constraints for a valid VM state
    s = vm_state_solver()

    # add overlapping constraint
    s.add(And(c_i, c_j))

    # Check for satisfiability
    if s.check() == sat:
        model = s.model()
        print(f"Overlap Model: {model}")
        return True
    else:
        return False


# symbolically check whether the rules are deterministic, i.e.,
# check whether a state transition in the VM leads to a single result state
# (and not multiple ones).
def check_determinism(rules):
    # produce all distinct pair of rules (ignore order)
    deterministic = True
    for i in range(len(rules)):
        for j in range(i):
            # retrieve s distinct pair of rules
            (name_i, cond_i, effect_i) = rules[i]
            (name_j, cond_j, effect_j) = rules[j]
            if effect_i != effect_j:
                if is_overlapping(cond_i, cond_j):
                    print("=> Check rules "
                          + name_i
                          + " and "
                          + name_j)
                    print("\tRules overlap and make specification indeterministic.")
                    print("\t"+str(cond_i)+"\t"+str(cond_j))
                    deterministic = False
    return deterministic


# symbolically check whether the rules are deterministic, i.e.,
# check whether a state transition in the VM leads to a single result state
# (and not multiple ones)
def check_completeness(rules):
    # construct covering predicate by building a disjunction over all
    # rule conditions.
    if len(rules) > 0:
        (_, cover, _) = rules[0]
        for i in range(1, len(rules)):
            (_, cond, _) = rules[i]
            cover = Or(cover, cond)
    else:
        cover = False

    # build constraints for a valid VM state
    s = vm_state_solver()

    # add overlapping constraint
    s.add(Not(cover))

    # Check for satisfiability
    if s.check() == sat:
        model = s.model()
        print(f"\tModel: {model}")
        return False
    else:
        return True
  
# open rule file and evaluate
#
# NB: a rule file can be generated with the following driver command:
#   go run ./go/ct/driver/ smt-printer --filter 'add_regular|mul_regular' >py/rules
# generating the successful add and mul operation.

# argument handling (TODO: replace with getopt style and help)
determinism = False
completeness = False
if len(sys.argv) == 2:
    determinism = True
    completeness = True
    fname = sys.argv[1]
elif len(sys.argv) == 3:
    fname = sys.argv[1]
    if sys.argv[2] == "determinism":
        determinism = True
    elif sys.argv[2] == "completeness":
        completeness = True
    else:
        print("Unknown type check")
        exit(1)
else:
    print("error: expect a rule file (and type check [determinism|completeness) as arguments.")
    exit(1)

# read rule file and convert it to a python object
try:
    print("Read specification ...")
    with open(fname, "r") as file:
        data = file.read()
except FileNotFoundError:
    print("error: rule file not found.")
rules = eval(data.replace("true", "True").replace("false","False"))

# perform determinism check
if determinism: 
    print("Check determinism ...")
    if check_determinism(rules):
        print("\tSpecification is deterministic.")
    else:
        print("\tSpecification is not deterministic.")

# perform completness check
if completeness:
    print("\nCheck completeness ...")
    if check_completeness(rules):
        print("\tSpecification is complete.")
    else:
        print("\tSpecification is not complete.")
