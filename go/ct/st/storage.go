// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package st

import (
	"fmt"

	"golang.org/x/exp/maps"

	"github.com/0xsoniclabs/Tosca/go/ct/common"
)

type Storage struct {
	current  map[common.U256]common.U256
	original map[common.U256]common.U256
	warm     map[common.U256]bool
}

type StorageBuilder struct {
	s Storage
}

func NewStorageBuilder() *StorageBuilder {
	return &StorageBuilder{}
}

func (s *StorageBuilder) Build() *Storage {
	res := s.s
	s.s = Storage{}

	return &res
}

func (s *StorageBuilder) SetCurrent(key, value common.U256) *StorageBuilder {
	if s.s.current == nil {
		s.s.current = make(map[common.U256]common.U256)
	}
	s.s.current[key] = value
	return s
}

func (s *StorageBuilder) SetOriginal(key, value common.U256) *StorageBuilder {
	if s.s.original == nil {
		s.s.original = make(map[common.U256]common.U256)
	}
	s.s.original[key] = value
	return s
}

func (s *StorageBuilder) SetWarm(key common.U256, value bool) *StorageBuilder {
	if value {
		if s.s.warm == nil {
			s.s.warm = make(map[common.U256]bool)
		}
		s.s.warm[key] = value
	}
	return s
}

func (s *StorageBuilder) IsInOriginal(key common.U256) bool {
	_, isIn := s.s.original[key]
	return isIn
}

func (s *Storage) SetCurrent(key common.U256, value common.U256) {
	if s.current == nil {
		s.current = make(map[common.U256]common.U256)
	} else {
		s.current = maps.Clone(s.current)
	}
	s.current[key] = value
}

func (s *Storage) GetCurrent(key common.U256) common.U256 {
	return s.current[key]
}

func (s *Storage) RemoveCurrent(key common.U256) {
	if s.current != nil {
		s.current = maps.Clone(s.current)
	}
	delete(s.current, key)
}

func (s *Storage) SetOriginal(key common.U256, value common.U256) {
	if s.original == nil {
		s.original = make(map[common.U256]common.U256)
	} else {
		s.original = maps.Clone(s.original)
	}
	s.original[key] = value
}

func (s *Storage) GetOriginal(key common.U256) common.U256 {
	return s.original[key]
}

func (s *Storage) RemoveOriginal(key common.U256) {
	if s.original != nil {
		s.original = maps.Clone(s.original)
	}
	delete(s.original, key)
}

func (s *Storage) IsWarm(key common.U256) bool {
	return s.warm[key]
}

func (s *Storage) MarkWarm(key common.U256) {
	if s.warm == nil {
		s.warm = make(map[common.U256]bool)
	} else {
		s.warm = maps.Clone(s.warm)
	}
	s.warm[key] = true
}

func (s *Storage) MarkCold(key common.U256) {
	if s.warm != nil {
		s.warm = maps.Clone(s.warm)
	}
	delete(s.warm, key)
}

func (s *Storage) Clone() *Storage {
	return &Storage{
		current:  s.current,
		original: s.original,
		warm:     s.warm,
	}
}

func mapEqualIgnoringZeroValues[K comparable](a map[K]common.U256, b map[K]common.U256) bool {
	for key, valueA := range a {
		valueB, contained := b[key]
		if !contained && !valueA.IsZero() {
			return false
		} else if valueA != valueB {
			return false
		}
	}
	for key, valueB := range b {
		if _, contained := a[key]; !contained && !valueB.IsZero() {
			return false
		}
	}
	return true
}

func (a *Storage) Eq(b *Storage) bool {
	return mapEqualIgnoringZeroValues(a.current, b.current) &&
		maps.Equal(a.original, b.original) &&
		maps.Equal(a.warm, b.warm)
}

func mapDiffIgnoringZeroValues[K comparable](a map[K]common.U256, b map[K]common.U256, name string) (res []string) {
	for key, valueA := range a {
		valueB, contained := b[key]
		if !contained && !valueA.IsZero() {
			res = append(res, fmt.Sprintf("Different %s entry:\n\t[%v]=%v\n\tvs\n\tmissing", name, key, valueA))
		} else if valueA != valueB {
			res = append(res, fmt.Sprintf("Different %s entry:\n\t[%v]=%v\n\tvs\n\t[%v]=%v", name, key, valueA, key, valueB))
		}
	}
	for key, valueB := range b {
		if _, contained := a[key]; !contained && !valueB.IsZero() {
			res = append(res, fmt.Sprintf("Different %s entry:\n\tmissing\n\tvs\n\t[%v]=%v", name, key, valueB))
		}
	}

	return
}

func (a *Storage) Diff(b *Storage) (res []string) {
	res = append(res, mapDiffIgnoringZeroValues(a.current, b.current, "current")...)
	res = append(res, mapDiffIgnoringZeroValues(a.original, b.original, "original")...)

	for key := range a.warm {
		if _, contained := b.warm[key]; !contained {
			res = append(res, fmt.Sprintf("Different warm entry: %v vs missing", key))
		}
	}
	for key := range b.warm {
		if _, contained := a.warm[key]; !contained {
			res = append(res, fmt.Sprintf("Different warm entry: missing vs %v", key))
		}
	}

	return
}
