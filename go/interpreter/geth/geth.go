// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package geth

import (
	"errors"
	"fmt"
	"math/big"

	ct "github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/geth_adapter"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/tracing"
	geth "github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/params"
	"github.com/holiman/uint256"
)

func init() {
	tosca.MustRegisterInterpreterFactory("geth", func(any) (tosca.Interpreter, error) {
		return &gethVm{}, nil
	})
}

type gethVm struct{}

// Defines the newest supported revision for this interpreter implementation
const newestSupportedRevision = tosca.R14_Prague

func (m *gethVm) Run(parameters tosca.Parameters) (tosca.Result, error) {
	if parameters.Revision > newestSupportedRevision {
		return tosca.Result{}, &tosca.ErrUnsupportedRevision{Revision: parameters.Revision}
	}
	evm, contract, stateDb := createGethInterpreterContext(parameters)

	output, err := evm.Interpreter().Run(contract, parameters.Input, false)

	result := tosca.Result{
		Output:    output,
		GasLeft:   tosca.Gas(contract.Gas),
		GasRefund: tosca.Gas(stateDb.GetRefund()),
		Success:   true,
	}

	// If no error is reported, the execution ended with a STOP, RETURN, or SUICIDE.
	if err == nil {
		return result, nil
	}

	// In case of a revert the result should indicate an unsuccessful execution.
	if err == geth.ErrExecutionReverted {
		result.Success = false
		return result, nil
	}

	// In case of an issue caused by the code execution, the result should indicate
	// a failed execution but no error should be reported.
	switch {
	case errors.Is(err, geth.ErrOutOfGas),
		errors.Is(err, geth.ErrCodeStoreOutOfGas),
		errors.Is(err, geth.ErrDepth),
		errors.Is(err, geth.ErrInsufficientBalance),
		errors.Is(err, geth.ErrContractAddressCollision),
		errors.Is(err, geth.ErrExecutionReverted),
		errors.Is(err, geth.ErrMaxCodeSizeExceeded),
		errors.Is(err, geth.ErrInvalidJump),
		errors.Is(err, geth.ErrWriteProtection),
		errors.Is(err, geth.ErrReturnDataOutOfBounds),
		errors.Is(err, geth.ErrReturnDataOutOfBounds),
		errors.Is(err, geth.ErrGasUintOverflow),
		errors.Is(err, geth.ErrInvalidCode):
		return tosca.Result{Success: false}, nil
	}

	if _, ok := err.(*geth.ErrStackOverflow); ok {
		return tosca.Result{Success: false}, nil
	}
	if _, ok := err.(*geth.ErrStackUnderflow); ok {
		return tosca.Result{Success: false}, nil
	}
	if _, ok := err.(*geth.ErrInvalidOpCode); ok {
		return tosca.Result{Success: false}, nil
	}

	// In all other cases an EVM error should be reported.
	return tosca.Result{}, fmt.Errorf("internal EVM error in geth: %v", err)
}

// MakeChainConfig returns a chain config for the given chain ID and target revision.
// The baseline config is used as a starting point, so that any prefilled configuration from go-ethereum:params/config.go can be used.
// chainId needs to be prefilled as it may be accessed with the opcode CHAINID.
// the fork-blocks and the fork-times are set to the respective values for the given revision.
func MakeChainConfig(baseline params.ChainConfig, chainId *big.Int, targetRevision tosca.Revision) params.ChainConfig {
	istanbulBlock := ct.GetForkBlock(tosca.R07_Istanbul)
	berlinBlock := ct.GetForkBlock(tosca.R09_Berlin)
	londonBlock := ct.GetForkBlock(tosca.R10_London)
	parisBlock := ct.GetForkBlock(tosca.R11_Paris)
	shanghaiTime := ct.GetForkTime(tosca.R12_Shanghai)
	cancunTime := ct.GetForkTime(tosca.R13_Cancun)
	pragueTime := ct.GetForkTime(tosca.R14_Prague)

	chainConfig := baseline
	chainConfig.ChainID = chainId
	chainConfig.ByzantiumBlock = big.NewInt(0)
	chainConfig.IstanbulBlock = big.NewInt(0).SetUint64(istanbulBlock)
	chainConfig.BerlinBlock = big.NewInt(0).SetUint64(berlinBlock)
	chainConfig.LondonBlock = big.NewInt(0).SetUint64(londonBlock)

	if targetRevision >= tosca.R11_Paris {
		chainConfig.MergeNetsplitBlock = big.NewInt(0).SetUint64(parisBlock)
	}
	if targetRevision >= tosca.R12_Shanghai {
		chainConfig.ShanghaiTime = &shanghaiTime
	}
	if targetRevision >= tosca.R13_Cancun {
		chainConfig.CancunTime = &cancunTime
	}
	if targetRevision >= tosca.R14_Prague {
		chainConfig.PragueTime = &pragueTime
	}

	return chainConfig
}

func currentBlock(revision tosca.Revision) *big.Int {
	block := ct.GetForkBlock(revision)
	return big.NewInt(int64(block + 2))
}

func createGethInterpreterContext(parameters tosca.Parameters) (*geth.EVM, *geth.Contract, *geth_adapter.StateDB) {
	// Set hard forks for chainconfig
	chainConfig :=
		MakeChainConfig(*params.AllEthashProtocolChanges,
			new(big.Int).SetBytes(parameters.ChainID[:]),
			parameters.Revision)

	// Hashing function used in the context for BLOCKHASH instruction
	getHash := func(num uint64) common.Hash {
		return common.Hash(parameters.Context.GetBlockHash(int64(num)))
	}

	// Create empty block context based on block number
	blockCtx := geth.BlockContext{
		BlockNumber: currentBlock(parameters.Revision),
		Time:        uint64(parameters.Timestamp),
		Difficulty:  big.NewInt(1),
		GasLimit:    uint64(parameters.GasLimit),
		GetHash:     getHash,
		BaseFee:     new(big.Int).SetBytes(parameters.BaseFee[:]),
		BlobBaseFee: new(big.Int).SetBytes(parameters.BlobBaseFee[:]),
		Transfer:    transferFunc,
		CanTransfer: canTransferFunc,
	}

	if parameters.Revision >= tosca.R11_Paris {
		// Setting the random signals to geth that a post-merge (Paris) revision should be utilized.
		hash := common.BytesToHash(parameters.PrevRandao[:])
		blockCtx.Random = &hash
	}

	// Create empty tx context
	txCtx := geth.TxContext{
		GasPrice:   new(big.Int).SetBytes(parameters.GasPrice[:]),
		BlobFeeCap: new(big.Int).SetBytes(parameters.BlobBaseFee[:]),
	}

	for _, hash := range parameters.BlobHashes {
		txCtx.BlobHashes = append(txCtx.BlobHashes, common.Hash(hash))
	}

	// Set interpreter variant for this VM
	config := geth.Config{}

	stateDb := geth_adapter.NewStateDB(parameters.Context)
	evm := geth.NewEVM(blockCtx, stateDb, &chainConfig, config)
	evm.TxContext = txCtx

	evm.Origin = common.Address(parameters.Origin)
	evm.Context.BlockNumber = big.NewInt(parameters.BlockNumber)
	evm.Context.Coinbase = common.Address(parameters.Coinbase)
	evm.Context.Difficulty = new(big.Int).SetBytes(parameters.PrevRandao[:])
	evm.Context.Time = uint64(parameters.Timestamp)

	value := parameters.Value.ToUint256()
	addr := geth.AccountRef(parameters.Recipient)
	contract := geth.NewContract(addr, addr, value, uint64(parameters.Gas))
	contract.CallerAddress = common.Address(parameters.Sender)
	contract.Code = parameters.Code
	contract.CodeHash = crypto.Keccak256Hash(parameters.Code)
	contract.Input = parameters.Input

	return evm, contract, stateDb
}

// --- Adapter ---

// transferFunc subtracts amount from sender and adds amount to recipient using the given Db
// Now is doing nothing as this is not changing gas computation
func transferFunc(stateDB geth.StateDB, callerAddress common.Address, to common.Address, value *uint256.Int) {
	// Can be something like this:
	stateDB.SubBalance(callerAddress, value, tracing.BalanceChangeTransfer)
	stateDB.AddBalance(to, value, tracing.BalanceChangeTransfer)
}

// canTransferFunc is the signature of a transfer function
func canTransferFunc(stateDB geth.StateDB, callerAddress common.Address, value *uint256.Int) bool {
	return stateDB.GetBalance(callerAddress).Cmp(value) >= 0
}
