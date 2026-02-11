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
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
	lru "github.com/hashicorp/golang-lru/v2"
)

// analysis is a cache for jump destination analyses of smart contract codes.
type analysis struct {
	cache             *lru.Cache[tosca.Hash, jumpDestMap]
	maxCachedCodeSize int
}

// newAnalysis creates a new analysis cache with the given size and maximum cached code size.
func newAnalysis(sizeInByte int, maxCachedCodeSize int) analysis {
	// convert the cache size in bytes to the number of entries
	size := (sizeInByte / maxCachedCodeSize) * 8 // each instruction requires 1 bit in the jumpDestMap

	cache, err := lru.New[tosca.Hash, jumpDestMap](size)
	if err != nil {
		panic("failed to create analysis cache: " + err.Error())
	}

	return analysis{cache: cache, maxCachedCodeSize: maxCachedCodeSize}
}

// analyzeJumpDest analyzes the given code for jump destinations. If a cache
// is available and the code hash is provided, it attempts to retrieve a cached
// analysis. If no cached analysis is found and the code length is within the
// allowed limit, it caches the new analysis before returning it.
func (a *analysis) analyzeJumpDest(code tosca.Code, codehash *tosca.Hash) jumpDestMap {
	if a.cache == nil || codehash == nil {
		return findJumpDestinations(code)
	}

	if analysis, ok := a.cache.Get(*codehash); ok {
		return analysis
	}

	if len(code) > a.maxCachedCodeSize {
		return findJumpDestinations(code)
	}

	jumpDests := findJumpDestinations(code)
	a.cache.Add(*codehash, jumpDests)
	return jumpDests
}

// jumpDestMap represents a bitmap of valid jump destinations within a smart contract code.
type jumpDestMap struct {
	bitmap   []uint64
	codeSize uint64
}

// newJumpDestMap creates a new jumpDestMap for the given code size.
func newJumpDestMap(size uint64) jumpDestMap {
	analysisSize := size / 64
	if size%64 != 0 {
		analysisSize++
	}
	analysis := jumpDestMap{
		bitmap:   make([]uint64, analysisSize),
		codeSize: size,
	}
	return analysis
}

// findJumpDestinations analyzes the given code and returns a jumpDestMap
// marking all valid jump destinations.
func findJumpDestinations(code tosca.Code) jumpDestMap {
	analysis := newJumpDestMap(uint64(len(code)))
	for idx := 0; idx < len(code); idx++ {
		op := vm.OpCode(code[idx])
		if op >= vm.PUSH1 && op <= vm.PUSH32 {
			// PUSH1 to PUSH32
			dataSize := int(op) - int(vm.PUSH1) + 1
			idx += dataSize // Skip the pushed data
			continue
		}
		if op == vm.JUMPDEST {
			analysis.markJumpDest(uint64(idx))
		}
	}
	return analysis
}

// isJumpDest checks if the given index is marked as a jump destination.
func (a *jumpDestMap) isJumpDest(idx uint64) bool {
	if a == nil {
		return false
	}
	uintIdx, mask := idxToAnalysisIdxAndMask(idx)
	if uintIdx >= uint64(len(a.bitmap)) {
		return false
	}
	return a.bitmap[uintIdx]&mask != 0
}

// markJumpDest marks the given index as a jump destination.
func (a *jumpDestMap) markJumpDest(idx uint64) {
	if idx >= uint64(a.codeSize) {
		return
	}

	uintIdx, mask := idxToAnalysisIdxAndMask(idx)
	if uintIdx >= uint64(len(a.bitmap)) {
		return
	}

	a.bitmap[uintIdx] |= mask
}

// idxToAnalysisIdxAndMask converts a code index to the corresponding
// index and bitmask in the jumpDestMap bitmap.
func idxToAnalysisIdxAndMask(idx uint64) (uint64, uint64) {
	return idx / 64, 1 << (idx % 64)
}
