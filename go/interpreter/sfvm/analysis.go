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

var analysisCache = createAnalysisCache(1 << 30) // = 1GiB

func createAnalysisCache(size int) *lru.Cache[tosca.Hash, Analysis] {
	cache, err := lru.New[tosca.Hash, Analysis](size)
	if err != nil {
		panic("failed to create analysis cache: " + err.Error())
	}
	return cache
}

func analyzeJumpDest(code tosca.Code, codehash *tosca.Hash) Analysis {
	if analysisCache == nil || codehash == nil {
		return jumpDestAnalysisInternal(code)
	}

	if analysis, ok := analysisCache.Get(*codehash); ok {
		return analysis
	}

	jumpDests := jumpDestAnalysisInternal(code)
	analysisCache.Add(*codehash, jumpDests)
	return jumpDests
}

type Analysis struct {
	jumpDestBitmap []uint64
	codeSize       uint64
}

func newAnalysis(size uint64) Analysis {
	analysisSize := size/64 + 1
	analysis := Analysis{
		jumpDestBitmap: make([]uint64, analysisSize),
		codeSize:       size,
	}
	return analysis
}

func jumpDestAnalysisInternal(code tosca.Code) Analysis {
	analysis := newAnalysis(uint64(len(code)))
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

func (a *Analysis) isJumpDest(idx uint64) bool {
	if idx >= a.codeSize {
		return false
	}
	uintIdx, mask := idxToAnalysisIdxAndMask(idx)
	return a.jumpDestBitmap[uintIdx]&mask != 0
}

func (a *Analysis) markJumpDest(idx uint64) {
	if idx >= uint64(a.codeSize) {
		return
	}
	uintIdx, mask := idxToAnalysisIdxAndMask(idx)
	a.jumpDestBitmap[uintIdx] |= mask
}

func idxToAnalysisIdxAndMask(idx uint64) (uint64, uint64) {
	return idx / 64, 1 << (idx % 64)
}
