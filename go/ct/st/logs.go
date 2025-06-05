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
	"slices"

	. "github.com/0xsoniclabs/tosca/go/ct/common"
)

type Logs struct {
	Entries []LogEntry
}

type LogEntry struct {
	Topics []U256
	Data   []byte
}

func NewLogs() *Logs {
	return &Logs{}
}

func (l *Logs) Clone() *Logs {
	clone := NewLogs()
	for _, entry := range l.Entries {
		clone.AddLog(entry.Data, entry.Topics...)
	}
	return clone
}

func (l *Logs) AddLog(data []byte, topics ...U256) {
	l.Entries = append(l.Entries, LogEntry{
		slices.Clone(topics),
		slices.Clone(data),
	})
}

func (l *Logs) Eq(other *Logs) bool {
	if len(l.Entries) != len(other.Entries) {
		return false
	}
	for i, aEntry := range l.Entries {
		bEntry := other.Entries[i]
		if !slices.Equal(aEntry.Topics, bEntry.Topics) {
			return false
		}
		if !slices.Equal(aEntry.Data, bEntry.Data) {
			return false
		}
	}
	return true
}

func (l *Logs) Diff(other *Logs) (res []string) {
	if len(l.Entries) != len(other.Entries) {
		res = append(res, fmt.Sprintf("Different log count: %v vs %v", len(l.Entries), len(other.Entries)))
	}

	minLength := len(l.Entries)
	if len(other.Entries) < minLength {
		minLength = len(other.Entries)
	}

	for i := 0; i < minLength; i++ {
		aEntry, bEntry := l.Entries[i], other.Entries[i]
		if !slices.Equal(aEntry.Topics, bEntry.Topics) {
			res = append(res, fmt.Sprintf("Different topics for log entry %d:\n\t%x\n\tvs\n\t%x\n", i, aEntry.Topics, bEntry.Topics))
		}
		if !slices.Equal(aEntry.Data, bEntry.Data) {
			res = append(res, fmt.Sprintf("Different data for log entry %d:\n\t%x\n\tvs\n\t%x\n", i, aEntry.Data, bEntry.Data))
		}
	}

	return
}
