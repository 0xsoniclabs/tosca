// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package sfvm

import (
	"math"
	"unsafe"

	"github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/tosca"

	"github.com/0xsoniclabs/tosca/go/tosca/vm"
	lru "github.com/hashicorp/golang-lru/v2"
)

// ConversionConfig contains a set of configuration options for the code conversion.
type ConversionConfig struct {
	// CacheSize is the maximum size of the maintained code cache in bytes.
	// If set to 0, a default size is used. If negative, no cache is used.
	// Cache sizes are grown in increments of maxCachedCodeLength.
	// Positive values larger than 0 but less than maxCachedCodeLength are
	// reported as invalid cache sizes during initialization.
	CacheSize int
}

// Converter converts EVM code to SFVM code.
type Converter struct {
	config ConversionConfig
	cache  *lru.Cache[tosca.Hash, Code]
}

// NewConverter creates a new code converter with the provided configuration.
func NewConverter(config ConversionConfig) (*Converter, error) {
	if config.CacheSize == 0 {
		config.CacheSize = (1 << 30) // = 1GiB
	}

	var cache *lru.Cache[tosca.Hash, Code]
	if config.CacheSize > 0 {
		var err error
		const instructionSize = int(unsafe.Sizeof(Instruction{}))
		capacity := config.CacheSize / maxCachedCodeLength / instructionSize
		cache, err = lru.New[tosca.Hash, Code](capacity)
		if err != nil {
			return nil, err
		}
	}
	return &Converter{
		config: config,
		cache:  cache,
	}, nil
}

// Convert converts EVM code to SFVM code. If the provided code hash is not nil,
// it is assumed to be a valid hash of the code and is used to cache the
// conversion result. If the hash is nil, the conversion result is not cached.
func (c *Converter) Convert(code []byte, codeHash *tosca.Hash) (Code, error) {
	if len(code) > math.MaxUint16 {
		return Code{}, errCodeSizeExceeded
	}

	if c.cache == nil || codeHash == nil {
		return convert(code, c.config), nil
	}

	res, exists := c.cache.Get(*codeHash)
	if exists {
		return res, nil
	}

	res = convert(code, c.config)
	if len(res) > maxCachedCodeLength {
		return res, nil
	}

	c.cache.Add(*codeHash, res)
	return res, nil
}

// maxCachedCodeLength is the maximum length of a code in bytes that are
// retained in the cache. To avoid excessive memory usage, longer codes are not
// cached. The defined limit is the current limit for codes stored on the chain.
// Only initialization codes can be longer. Since the Shanghai hard fork, the
// maximum size of initialization codes is 2 * 24_576 = 49_152 bytes (see
// https://eips.ethereum.org/EIPS/eip-3860). Such init codes are deliberately
// not cached due to the expected limited re-use and the missing code hash.
const maxCachedCodeLength = 1<<14 + 1<<13 // = 24_576 bytes

// --- code builder ---

type codeBuilder struct {
	code    []Instruction
	nextPos int
}

func newCodeBuilder(codelength int) codeBuilder {
	return codeBuilder{make([]Instruction, codelength), 0}
}

func (b *codeBuilder) length() int {
	return b.nextPos
}

func (b *codeBuilder) appendOp(opcode OpCode, arg uint16) *codeBuilder {
	b.code[b.nextPos].opcode = opcode
	b.code[b.nextPos].arg = arg
	b.nextPos++
	return b
}

func (b *codeBuilder) appendCode(opcode OpCode) *codeBuilder {
	b.code[b.nextPos].opcode = opcode
	b.nextPos++
	return b
}

func (b *codeBuilder) appendData(data uint16) *codeBuilder {
	return b.appendOp(DATA, data)
}

func (b *codeBuilder) padNoOpsUntil(pos int) {
	for i := b.nextPos; i < pos; i++ {
		b.code[i].opcode = NOOP
	}
	b.nextPos = pos
}

func (b *codeBuilder) toCode() Code {
	return b.code[0:b.nextPos]
}

func convert(code []byte, options ConversionConfig) Code {
	return convertWithObserver(code, options, func(int, int) {})
}

// convertWithObserver converts EVM code to SFVM code and calls the observer
// with the code position of every pair of instructions converted.
func convertWithObserver(
	code []byte,
	options ConversionConfig,
	observer func(evmPc int, sfvmPc int),
) Code {
	res := newCodeBuilder(len(code))

	// Convert each individual instruction.
	for i := 0; i < len(code); {
		// Handle jump destinations
		if code[i] == byte(vm.JUMPDEST) {
			// Jump to the next jump destination and fill space with noops
			if res.length() < i {
				res.appendOp(JUMP_TO, uint16(i))
			}
			res.padNoOpsUntil(i)
			res.appendCode(JUMPDEST)
			observer(i, i)
			i++
			continue
		}

		// Convert instructions
		observer(i, res.nextPos)
		inc := appendInstructions(&res, i, code)
		i += inc + 1
	}
	return res.toCode()
}

func appendInstructions(res *codeBuilder, pos int, code []byte) int {
	// Convert individual instructions.
	toscaOpCode := vm.OpCode(code[pos])

	if toscaOpCode == vm.PC {
		if pos > math.MaxUint16 {
			res.appendCode(INVALID)
			return 1
		}
		res.appendOp(PC, uint16(pos))
		return 0
	}

	if vm.PUSH1 <= toscaOpCode && toscaOpCode <= vm.PUSH32 {
		// Determine the number of bytes to be pushed.
		numBytes := int(toscaOpCode) - int(vm.PUSH1) + 1

		var data []byte
		// If there are not enough bytes left in the code, rest is filled with 0
		// zeros are padded right
		if len(code) < pos+numBytes+2 {
			extension := (pos + numBytes + 2 - len(code)) / 2
			if (pos+numBytes+2-len(code))%2 > 0 {
				extension++
			}
			if extension > 0 {
				instruction := common.RightPadSlice(res.code[:], len(res.code)+extension)
				res.code = instruction
			}
			data = common.RightPadSlice(code[pos+1:], numBytes+1)
		} else {
			data = code[pos+1 : pos+1+numBytes]
		}

		// Fix the op-codes of the resulting instructions
		if numBytes == 1 {
			res.appendOp(PUSH1, uint16(data[0])<<8)
		} else {
			res.appendOp(PUSH1+OpCode(numBytes-1), uint16(data[0])<<8|uint16(data[1]))
		}

		// Fix the arguments by packing them in pairs into the instructions.
		for i := 2; i < numBytes-1; i += 2 {
			res.appendData(uint16(data[i])<<8 | uint16(data[i+1]))
		}
		if numBytes > 1 && numBytes%2 == 1 {
			res.appendData(uint16(data[numBytes-1]) << 8)
		}

		return numBytes
	}

	// All the rest converts to a single instruction.
	res.appendCode(OpCode(toscaOpCode))
	return 0
}
