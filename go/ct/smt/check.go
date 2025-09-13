package smt

import (
	_ "embed"
	"os"
	"slices"
	"strings"
	"text/template"

	"github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/ct/rlz"
	"github.com/0xsoniclabs/tosca/go/ct/spc"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
)

func CheckCompleteness(spec spc.Specification) error {
	path := "out.smt2"
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	defer f.Close()

	rules := spec.GetRules()

	seen := map[string]struct{}{}
	uniqueRules := make([]rlz.Rule, 0, len(rules))
	for _, r := range rules {
		if _, ok := seen[r.Name]; ok {
			continue
		}
		seen[r.Name] = struct{}{}
		uniqueRules = append(uniqueRules, r)
	}
	rules = uniqueRules

	slices.SortFunc(rules, func(a, b rlz.Rule) int {
		return strings.Compare(a.Name, b.Name)
	})

	revisions := []tosca.Revision{}
	for rev := common.MinRevision; rev <= common.NewestSupportedRevision; rev++ {
		revisions = append(revisions, rev)
	}

	codes := make([]vm.OpCode, 0, 256)
	for i := range 256 {
		cur := vm.OpCode(i)
		if vm.IsValid(cur) {
			codes = append(codes, cur)
		}
	}
	codes = append(codes, vm.INVALID)

	input := input{
		Rules:     rules,
		Revisions: revisions,
		OpCodes:   codes,
	}

	return completenessTemplate.Execute(f, input)
}

type input struct {
	Rules     []rlz.Rule
	Revisions []tosca.Revision
	OpCodes   []vm.OpCode
}

//go:embed completeness.smt2
var rawCompletenessSpec string
var completenessTemplate = template.Must(
	template.New("completeness").Funcs(funcMap).Parse(rawCompletenessSpec),
)

var funcMap = template.FuncMap{
	"escapeLiteral": func(str string) string {
		str = strings.ReplaceAll(str, `(`, `_`)
		str = strings.ReplaceAll(str, `)`, `_`)
		return str
	},
	"revisionToInt": func(rev tosca.Revision) int {
		return int(rev)
	},
	"opToInt": func(op vm.OpCode) int {
		return int(op)
	},
}
