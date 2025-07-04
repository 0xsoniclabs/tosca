// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

// Code generated by MockGen. DO NOT EDIT.
// Source: interpreter.go
//
// Generated by this command:
//
//	mockgen -source interpreter.go -destination interpreter_mock.go -package lfvm
//

// Package lfvm is a generated GoMock package.
package lfvm

import (
	reflect "reflect"

	gomock "go.uber.org/mock/gomock"
)

// Mockrunner is a mock of runner interface.
type Mockrunner struct {
	ctrl     *gomock.Controller
	recorder *MockrunnerMockRecorder
}

// MockrunnerMockRecorder is the mock recorder for Mockrunner.
type MockrunnerMockRecorder struct {
	mock *Mockrunner
}

// NewMockrunner creates a new mock instance.
func NewMockrunner(ctrl *gomock.Controller) *Mockrunner {
	mock := &Mockrunner{ctrl: ctrl}
	mock.recorder = &MockrunnerMockRecorder{mock}
	return mock
}

// EXPECT returns an object that allows the caller to indicate expected use.
func (m *Mockrunner) EXPECT() *MockrunnerMockRecorder {
	return m.recorder
}

// run mocks base method.
func (m *Mockrunner) run(arg0 *context) (status, error) {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "run", arg0)
	ret0, _ := ret[0].(status)
	ret1, _ := ret[1].(error)
	return ret0, ret1
}

// run indicates an expected call of run.
func (mr *MockrunnerMockRecorder) run(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "run", reflect.TypeOf((*Mockrunner)(nil).run), arg0)
}
