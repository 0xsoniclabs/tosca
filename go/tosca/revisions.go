// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package tosca

import (
	"encoding/json"
	"fmt"
	"regexp"
	"strconv"
)

func (r Revision) String() string {
	switch r {
	case R07_Istanbul:
		return "Istanbul"
	case R09_Berlin:
		return "Berlin"
	case R10_London:
		return "London"
	case R11_Paris:
		return "Paris"
	case R12_Shanghai:
		return "Shanghai"
	case R13_Cancun:
		return "Cancun"
	case R14_Prague:
		return "Prague"
	default:
		return fmt.Sprintf("Revision(%d)", r)
	}
}

func GetAllKnownRevisions() []Revision {
	res := []Revision{}
	for i := 0; i < numRevisions; i++ {
		res = append(res, Revision(i))
	}
	return res
}

func (r Revision) MarshalJSON() ([]byte, error) {
	revString := r.String()
	return json.Marshal(revString)
}

func (r *Revision) UnmarshalJSON(data []byte) error {
	var s string
	err := json.Unmarshal(data, &s)
	if err != nil {
		return err
	}
	var revision Revision

	switch s {
	case "Istanbul":
		revision = R07_Istanbul
	case "Berlin":
		revision = R09_Berlin
	case "London":
		revision = R10_London
	case "Paris":
		revision = R11_Paris
	case "Shanghai":
		revision = R12_Shanghai
	case "Cancun":
		revision = R13_Cancun
	case "Prague":
		revision = R14_Prague
	default:
		// read Revision(X) format and extract the number.
		reg := regexp.MustCompile(`Revision\(([0-9]+)\)`)
		substring := reg.FindAllStringSubmatch(s, 1)
		if substring == nil {
			return &json.UnmarshalTypeError{}
		}
		revNumber := substring[0][1]
		revInt, err := strconv.Atoi(revNumber)
		if err != nil {
			return err
		}
		revision = Revision(revInt)
	}

	*r = revision
	return nil
}
