// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package geth

import (
	"fmt"
	"slices"
	"testing"

	"github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/ct/spc"
	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
)

// TestCtAdapter_Eip8038RulesMatchGeth cross-checks the conformance test rules
// pricing the access to durable state in Amsterdam against geth, the reference
// implementation of EIP-8038. Covered is one state per instruction and per
// condition the prices distinguish.
func TestCtAdapter_Eip8038RulesMatchGeth(t *testing.T) {
	for name, input := range eip8038States() {
		t.Run(name, func(t *testing.T) {
			defer input.Release()

			rules := spc.Spec.GetRulesFor(input)
			if len(rules) == 0 {
				t.Fatalf("no rule covers %v", input)
			}

			want := input.Clone()
			defer want.Release()
			rules[0].Effect.Apply(want)

			got, err := ctAdapter{}.StepN(input.Clone(), 1)
			if err != nil {
				t.Fatalf("failed to run geth: %v", err)
			}
			defer got.Release()

			// The rules do not model the return data of a step, and geth does
			// not report a program counter for halted runs.
			got.ReturnData = want.ReturnData
			if got.Status != st.Running {
				got.Pc = want.Pc
			}

			if !got.Eq(want) {
				t.Errorf("unexpected result, diff: %v", got.Diff(want))
			}
		})
	}
}

const eip8038Gas = tosca.Gas(1_000_000)

var (
	eip8038Self     = tosca.Address{0x01}
	eip8038Target   = tosca.Address{0x42}
	eip8038Delegate = tosca.Address{0x43}
)

// eip8038State creates a state about to execute the given instruction with the
// given operands, listed in the order the instruction pops them.
func eip8038State(op vm.OpCode, operands ...common.U256) *st.State {
	state := st.NewState(st.NewCode([]byte{byte(op)}))
	state.Status = st.Running
	state.Revision = tosca.R16_Amsterdam
	state.BlockContext.BlockNumber = common.GetForkBlock(tosca.R16_Amsterdam)
	state.Gas = eip8038Gas
	state.CallContext.AccountAddress = eip8038Self

	slices.Reverse(operands)
	state.Stack = st.NewStack(operands...)
	return state
}

func eip8038States() map[string]*st.State {
	states := map[string]*st.State{}
	add := func(name string, state *st.State) {
		if _, exists := states[name]; exists {
			panic("duplicate test state " + name)
		}
		states[name] = state
	}

	warmName := map[bool]string{false: "cold", true: "warm"}
	emptyName := map[bool]string{false: "funded_target", true: "empty_target"}
	successName := map[bool]string{false: "call_fails", true: "call_succeeds"}

	// --- accessing an account and its code ---

	for _, op := range []vm.OpCode{vm.BALANCE, vm.EXTCODESIZE, vm.EXTCODEHASH, vm.EXTCODECOPY} {
		for _, warm := range []bool{false, true} {
			for _, empty := range []bool{false, true} {
				operands := []common.U256{common.AddressToU256(eip8038Target)}
				if op == vm.EXTCODECOPY {
					operands = append(operands,
						common.NewU256(0),  // < destination offset in memory
						common.NewU256(0),  // < offset in the code
						common.NewU256(64), // < number of bytes to copy
					)
				}
				state := eip8038State(op, operands...)
				state.Accounts = eip8038Accounts(warm, empty, eip8038NoDelegation)
				add(fmt.Sprintf("%v/%s/%s", op, warmName[warm], emptyName[empty]), state)
			}
		}
	}

	// --- accessing a storage slot ---

	for _, warm := range []bool{false, true} {
		key := common.NewU256(1)
		state := eip8038State(vm.SLOAD, key)
		state.Storage = st.NewStorageBuilder().
			SetOriginal(key, common.NewU256(7)).
			SetCurrent(key, common.NewU256(7)).
			SetWarm(key, warm).
			Build()
		add(fmt.Sprintf("%v/%s", vm.SLOAD, warmName[warm]), state)
	}

	// The value a slot is committed to, currently holds, and is assigned, for
	// every configuration the price of a write distinguishes. A slot whose
	// current value deviates from its committed one has been written to before
	// and is therefore necessarily warm.
	sstoreConfigurations := map[tosca.StorageStatus][3]uint64{
		tosca.StorageAdded:            {0, 0, 1},
		tosca.StorageDeleted:          {1, 1, 0},
		tosca.StorageModified:         {1, 1, 2},
		tosca.StorageAssigned:         {1, 2, 3},
		tosca.StorageAddedDeleted:     {0, 1, 0},
		tosca.StorageDeletedAdded:     {1, 0, 2},
		tosca.StorageDeletedRestored:  {1, 0, 1},
		tosca.StorageModifiedDeleted:  {1, 2, 0},
		tosca.StorageModifiedRestored: {1, 2, 1},
	}
	for configuration, values := range sstoreConfigurations {
		original, current, assigned := values[0], values[1], values[2]
		for _, warm := range []bool{false, true} {
			if !warm && original != current {
				continue
			}
			key := common.NewU256(1)
			state := eip8038State(vm.SSTORE, key, common.NewU256(assigned))
			state.Storage = st.NewStorageBuilder().
				SetOriginal(key, common.NewU256(original)).
				SetCurrent(key, common.NewU256(current)).
				SetWarm(key, warm).
				Build()
			add(fmt.Sprintf("%v/%v/%s", vm.SSTORE, configuration, warmName[warm]), state)
		}
	}

	// --- calling another account ---

	for _, op := range []vm.OpCode{vm.CALL, vm.CALLCODE, vm.STATICCALL, vm.DELEGATECALL} {
		transfersValue := op == vm.CALL || op == vm.CALLCODE
		for _, value := range eip8038CallValues(transfersValue) {
			for _, warm := range []bool{false, true} {
				for _, empty := range []bool{false, true} {
					for _, delegation := range eip8038Delegations {
						for _, success := range []bool{false, true} {
							operands := []common.U256{
								common.NewU256(uint64(eip8038Gas)), // < gas limit of the call
								common.AddressToU256(eip8038Target),
							}
							if transfersValue {
								operands = append(operands, common.NewU256(value))
							}
							operands = append(operands,
								common.NewU256(0), // < input offset in memory
								common.NewU256(0), // < input size
								common.NewU256(0), // < output offset in memory
								common.NewU256(0), // < output size
							)
							state := eip8038State(op, operands...)
							state.Accounts = eip8038Accounts(warm, empty, delegation)
							state.CallJournal.Future = []st.FutureCall{{
								Success:  success,
								GasCosts: 100,
							}}

							add(fmt.Sprintf("%v/value_%d/%s/%s/%s/%s",
								op, value, warmName[warm], emptyName[empty],
								delegation, successName[success]), state)
						}
					}
				}
			}
		}
	}

	// --- creating an account ---

	for _, op := range []vm.OpCode{vm.CREATE, vm.CREATE2} {
		for _, value := range []uint64{0, 1, eip8038Balance + 1} {
			for _, success := range []bool{false, true} {
				operands := []common.U256{
					common.NewU256(value),
					common.NewU256(0),  // < offset of the init code in memory
					common.NewU256(64), // < size of the init code
				}
				if op == vm.CREATE2 {
					operands = append(operands, common.NewU256(0)) // < salt
				}
				state := eip8038State(op, operands...)
				state.Accounts = eip8038Accounts(false, true, eip8038NoDelegation)
				state.CallJournal.Future = []st.FutureCall{{
					Success:        success,
					GasCosts:       100,
					CreatedAccount: eip8038Target,
				}}
				add(fmt.Sprintf("%v/value_%d/%s", op, value, successName[success]), state)
			}
		}
	}

	// --- destroying an account ---

	fundedName := map[bool]string{false: "drained_originator", true: "funded_originator"}
	contractName := map[bool]string{false: "existing_contract", true: "new_contract"}

	for _, funded := range []bool{false, true} {
		for _, beneficiary := range eip8038Beneficiaries(funded) {
			for _, warm := range []bool{false, true} {
				for _, newContract := range beneficiary.newContracts {
					state := eip8038State(vm.SELFDESTRUCT, common.AddressToU256(beneficiary.address))
					state.IsNewContract = newContract
					state.Accounts = eip8038SelfDestructAccounts(
						beneficiary.address, funded, warm, beneficiary.empty)
					add(fmt.Sprintf("%v/%s/%s/%s/%s", vm.SELFDESTRUCT,
						fundedName[funded], beneficiary.name,
						warmName[warm], contractName[newContract]), state)
				}
			}
		}
	}

	return states
}

// eip8038Beneficiary is a beneficiary a self-destruct is tested against.
type eip8038Beneficiary struct {
	name    string
	address tosca.Address
	empty   bool

	// newContracts are the values of the is-new-contract flag to test the
	// beneficiary with, which decides whether the account is truly destroyed.
	newContracts []bool
}

// eip8038Beneficiaries lists the beneficiaries worth destroying an account in
// favor of, given whether that account holds funds. A separate account can be
// empty or not, whereas the executing account is empty exactly if it is drained.
func eip8038Beneficiaries(funded bool) []eip8038Beneficiary {
	both := []bool{false, true}
	return []eip8038Beneficiary{
		{"empty_beneficiary", eip8038Target, true, both},
		{"funded_beneficiary", eip8038Target, false, both},
		// Since EIP-8246 a self-destruct to self moves no balance, so it neither
		// burns funds nor emits a transfer log. Destroying the account is left
		// out: the geth adapter derives the beneficiary it journals from the
		// balance transfer, which does not happen in this case.
		{"self_beneficiary", eip8038Self, !funded, []bool{false}},
	}
}

// eip8038SelfDestructAccounts describes the world state of a self-destruct
// test: the executing account holding funds or not, and the beneficiary in the
// requested shape.
func eip8038SelfDestructAccounts(beneficiary tosca.Address, funded, warm, empty bool) *st.Accounts {
	accounts := st.NewAccountsBuilder()
	if funded {
		accounts.SetBalance(eip8038Self, common.NewU256(eip8038Balance))
	}
	if beneficiary != eip8038Self && !empty {
		accounts.SetBalance(beneficiary, common.NewU256(1))
		accounts.SetCode(beneficiary, common.NewBytes([]byte{byte(vm.STOP)}))
	}
	if warm {
		accounts.SetWarm(beneficiary)
	}
	return accounts.Build()
}

// eip8038Balance is the balance of the account executing the tested
// instruction, sufficient to fund a call transferring a value of one.
const eip8038Balance = 1

// The shapes of the code of the target of a call: plain code, or a designator
// delegating to an account that is either already warm or still cold.
const (
	eip8038NoDelegation   = "plain_target"
	eip8038ColdDelegation = "cold_delegating_target"
	eip8038WarmDelegation = "warm_delegating_target"
)

var eip8038Delegations = []string{eip8038NoDelegation, eip8038ColdDelegation, eip8038WarmDelegation}

// eip8038Accounts describes the world state of a test: the executing account
// holding funds, and the target account of the tested instruction in the
// requested shape.
func eip8038Accounts(warm, empty bool, delegation string) *st.Accounts {
	accounts := st.NewAccountsBuilder()
	accounts.SetBalance(eip8038Self, common.NewU256(eip8038Balance))
	if !empty {
		accounts.SetBalance(eip8038Target, common.NewU256(1))
	}
	switch delegation {
	case eip8038ColdDelegation, eip8038WarmDelegation:
		accounts.SetCode(eip8038Target, common.NewDelegationDesignator(eip8038Delegate))
		if delegation == eip8038WarmDelegation {
			accounts.SetWarm(eip8038Delegate)
		}
	default:
		if !empty {
			accounts.SetCode(eip8038Target, common.NewBytes([]byte{byte(vm.STOP)}))
		}
	}
	if warm {
		accounts.SetWarm(eip8038Target)
	}
	return accounts.Build()
}

// eip8038CallValues returns the values worth attaching to a call: none if the
// instruction cannot carry one, and otherwise no value, an affordable one, and
// one exceeding the balance of the executing account.
func eip8038CallValues(transfersValue bool) []uint64 {
	if !transfersValue {
		return []uint64{0}
	}
	return []uint64{0, eip8038Balance, eip8038Balance + 1}
}
