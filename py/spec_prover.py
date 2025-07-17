from cvc5.pythonic import *

################################################################
# Specification prover is an experimental prover for checking
# the correctness of the CT specification. The CT specification
# consists of a set of rules describing the transition from
# one state of the virtual machine to a new state. A rule is
# a triple consisting of name, condition, and effect.
#
# A state in the VM is denoted by "s" in the set of states "State".
# A condition for rule "r" is a function cond_r: State -> Bool
# that maps a state to a boolean value. The effect for a rule
# "r" is a function effect_r: State -> State that generates
# a new state assuming the that the condition of the rules hold.
#
# For an input state s, we compute a new state s', using the
# conditional functional construct:
#
#    for all r:
#       if cond_r(s), then s'=effect_r(s)
#
# The conditional functional construct and the relation format
# of the specification lead to questions about correctness. For
# example, what is if two or more conditions hold and their effect
# differ leading to different output states after the transition.
# The virtual machine would lose its deterministic behaviour.
# Another question arises related that for any virtual machine
# state, we have at least one rule, which can be applied.
#
# For a specification, we would like to prove two properties:
#
#   1) Determinism
#   2) Completeness
#
# The first property ensures that a transition produces only
# a single result state (rather than multiple ones). The second
# transition ensures that for any state there exists a transition.
#
# The determinism can be expressed as:
#   forall r<>r':
#     exists s:
#       cond_r(s) /\ cond_r'(s) => effect_r(s) = effect_r'(s)
#
# We simplify the property by abstraction. Instead of checking
# the effect, we use an unique name for different effect functions
# of rules. If we have two rules with the same effect name,
# we can assume that they have the same effect. If they have different
# names, we assume that they have a different effect, i.e., there
# exists a state such that the result state of both effect functions
# are different.
#
# This helps because the existing CT framework encodes the effects
# directly as GOLANG functions and we cannot export them as symbolic
# functions at the moment. Another aspect of this modelling is that
# property checking is substantially simplified since effect descriptions
# can be quite complex.
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

######################
# Construct VM State #
######################

# Virtual Machine Status
status = Int("status")
running = 0
stopped = 1
reverted = 2
failed = 3
NumStatusCodes = 4

# Program Counter
pc = Int("pc")
MaxCodeLen = 16384 + 8192

# Code Block
code_block = Array("code_block", IntSort(), IntSort())

# EVM Instructions
ADD = 3
MUL = 5


# Fetch operation on code block
def code(x):
    op = Int("op")
    op = Select(code_block, x)
    return op


# Gas
gas = Int("gas")

# stackSize
stackSize = Int("stackSize")

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

    # bound stack size
    s.add(stackSize >= 0, stackSize <= 1024)

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
                    print(
                        "=> Rule "
                        + name_i
                        + " and rule "
                        + name_j
                        + " make specification indeterministic."
                    )
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
            (_, cond, _) = rules[0]
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


print("Specification Checker")
print("=====================\n")

# open rule file and evaluate
#
# NB: a rule file can be generated with the following driver command:
#   go run ./go/ct/driver/ smt-printer --filter 'add_regular|mul_regular' >py/rules
# generating the successful add and mul operation.

if len(sys.argv) != 2:
    print("error: expect a rule file as argument.")
    exit(1)
try:
    with open(sys.argv[1], "r") as file:
        data = file.read()
except FileNotFoundError:
    print("error: rule file not found.")
rules = eval(data)

# perform determinism check
print("Check determinism?")
if check_determinism(rules):
    print("\tSpecification is deterministic.")
else:
    print("\tSpecification is not deterministic.")

# perform completness check
print("\nCheck completeness?")
if check_completeness(rules):
    print("\tSpecification is complete.")
else:
    print("\tSpecification is not complete.")
