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

type analysis struct {
	cache *lru.Cache[tosca.Hash, *jumpDestMap]
}

func newAnalysis(size int) analysis {
	cache, err := lru.New[tosca.Hash, *jumpDestMap](size)
	if err != nil {
		panic("failed to create analysis cache: " + err.Error())
	}
	return analysis{cache: cache}
}

func (a *analysis) analyzeJumpDest(code tosca.Code, codehash *tosca.Hash) *jumpDestMap {
	if a.cache == nil || codehash == nil {
		return jumpDestAnalysisInternal(code)
	}

	if analysis, ok := a.cache.Get(*codehash); ok {
		return analysis
	}

	jumpDests := jumpDestAnalysisInternal(code)
	a.cache.Add(*codehash, jumpDests)
	return jumpDests
}

type jumpDestMap struct {
	bitmap   []uint64
	codeSize uint64
}

func newJumpDestMap(size uint64) *jumpDestMap {
	analysisSize := size/64 + 1
	analysis := &jumpDestMap{
		bitmap:   make([]uint64, analysisSize),
		codeSize: size,
	}
	return analysis
}

func jumpDestAnalysisInternal(code tosca.Code) *jumpDestMap {
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

func (a *jumpDestMap) isJumpDest(idx uint64) bool {
	if a == nil {
		return false
	}
	if idx >= a.codeSize {
		return false
	}
	uintIdx, mask := idxToAnalysisIdxAndMask(idx)
	return a.bitmap[uintIdx]&mask != 0
}

func (a *jumpDestMap) markJumpDest(idx uint64) {
	if idx >= uint64(a.codeSize) {
		return
	}
	uintIdx, mask := idxToAnalysisIdxAndMask(idx)
	a.bitmap[uintIdx] |= mask
}

func idxToAnalysisIdxAndMask(idx uint64) (uint64, uint64) {
	return idx / 64, 1 << (idx % 64)
}
