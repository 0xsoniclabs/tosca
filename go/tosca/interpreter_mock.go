// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
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
//	mockgen -source interpreter.go -destination interpreter_mock.go -package tosca
//

// Package tosca is a generated GoMock package.
package tosca

import (
	reflect "reflect"

	gomock "go.uber.org/mock/gomock"
)

// MockInterpreter is a mock of Interpreter interface.
type MockInterpreter struct {
	ctrl     *gomock.Controller
	recorder *MockInterpreterMockRecorder
	isgomock struct{}
}

// MockInterpreterMockRecorder is the mock recorder for MockInterpreter.
type MockInterpreterMockRecorder struct {
	mock *MockInterpreter
}

// NewMockInterpreter creates a new mock instance.
func NewMockInterpreter(ctrl *gomock.Controller) *MockInterpreter {
	mock := &MockInterpreter{ctrl: ctrl}
	mock.recorder = &MockInterpreterMockRecorder{mock}
	return mock
}

// EXPECT returns an object that allows the caller to indicate expected use.
func (m *MockInterpreter) EXPECT() *MockInterpreterMockRecorder {
	return m.recorder
}

// Run mocks base method.
func (m *MockInterpreter) Run(arg0 Parameters) (Result, error) {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "Run", arg0)
	ret0, _ := ret[0].(Result)
	ret1, _ := ret[1].(error)
	return ret0, ret1
}

// Run indicates an expected call of Run.
func (mr *MockInterpreterMockRecorder) Run(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "Run", reflect.TypeOf((*MockInterpreter)(nil).Run), arg0)
}

// MockRunContext is a mock of RunContext interface.
type MockRunContext struct {
	ctrl     *gomock.Controller
	recorder *MockRunContextMockRecorder
	isgomock struct{}
}

// MockRunContextMockRecorder is the mock recorder for MockRunContext.
type MockRunContextMockRecorder struct {
	mock *MockRunContext
}

// NewMockRunContext creates a new mock instance.
func NewMockRunContext(ctrl *gomock.Controller) *MockRunContext {
	mock := &MockRunContext{ctrl: ctrl}
	mock.recorder = &MockRunContextMockRecorder{mock}
	return mock
}

// EXPECT returns an object that allows the caller to indicate expected use.
func (m *MockRunContext) EXPECT() *MockRunContextMockRecorder {
	return m.recorder
}

// AccessAccount mocks base method.
func (m *MockRunContext) AccessAccount(arg0 Address) AccessStatus {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "AccessAccount", arg0)
	ret0, _ := ret[0].(AccessStatus)
	return ret0
}

// AccessAccount indicates an expected call of AccessAccount.
func (mr *MockRunContextMockRecorder) AccessAccount(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "AccessAccount", reflect.TypeOf((*MockRunContext)(nil).AccessAccount), arg0)
}

// AccessStorage mocks base method.
func (m *MockRunContext) AccessStorage(arg0 Address, arg1 Key) AccessStatus {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "AccessStorage", arg0, arg1)
	ret0, _ := ret[0].(AccessStatus)
	return ret0
}

// AccessStorage indicates an expected call of AccessStorage.
func (mr *MockRunContextMockRecorder) AccessStorage(arg0, arg1 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "AccessStorage", reflect.TypeOf((*MockRunContext)(nil).AccessStorage), arg0, arg1)
}

// AccountExists mocks base method.
func (m *MockRunContext) AccountExists(arg0 Address) bool {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "AccountExists", arg0)
	ret0, _ := ret[0].(bool)
	return ret0
}

// AccountExists indicates an expected call of AccountExists.
func (mr *MockRunContextMockRecorder) AccountExists(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "AccountExists", reflect.TypeOf((*MockRunContext)(nil).AccountExists), arg0)
}

// Call mocks base method.
func (m *MockRunContext) Call(kind CallKind, parameter CallParameters) (CallResult, error) {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "Call", kind, parameter)
	ret0, _ := ret[0].(CallResult)
	ret1, _ := ret[1].(error)
	return ret0, ret1
}

// Call indicates an expected call of Call.
func (mr *MockRunContextMockRecorder) Call(kind, parameter any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "Call", reflect.TypeOf((*MockRunContext)(nil).Call), kind, parameter)
}

// CreateAccount mocks base method.
func (m *MockRunContext) CreateAccount(arg0 Address) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "CreateAccount", arg0)
}

// CreateAccount indicates an expected call of CreateAccount.
func (mr *MockRunContextMockRecorder) CreateAccount(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "CreateAccount", reflect.TypeOf((*MockRunContext)(nil).CreateAccount), arg0)
}

// CreateSnapshot mocks base method.
func (m *MockRunContext) CreateSnapshot() Snapshot {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "CreateSnapshot")
	ret0, _ := ret[0].(Snapshot)
	return ret0
}

// CreateSnapshot indicates an expected call of CreateSnapshot.
func (mr *MockRunContextMockRecorder) CreateSnapshot() *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "CreateSnapshot", reflect.TypeOf((*MockRunContext)(nil).CreateSnapshot))
}

// EmitLog mocks base method.
func (m *MockRunContext) EmitLog(arg0 Log) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "EmitLog", arg0)
}

// EmitLog indicates an expected call of EmitLog.
func (mr *MockRunContextMockRecorder) EmitLog(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "EmitLog", reflect.TypeOf((*MockRunContext)(nil).EmitLog), arg0)
}

// GetBalance mocks base method.
func (m *MockRunContext) GetBalance(arg0 Address) Value {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetBalance", arg0)
	ret0, _ := ret[0].(Value)
	return ret0
}

// GetBalance indicates an expected call of GetBalance.
func (mr *MockRunContextMockRecorder) GetBalance(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetBalance", reflect.TypeOf((*MockRunContext)(nil).GetBalance), arg0)
}

// GetBlockHash mocks base method.
func (m *MockRunContext) GetBlockHash(number int64) Hash {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetBlockHash", number)
	ret0, _ := ret[0].(Hash)
	return ret0
}

// GetBlockHash indicates an expected call of GetBlockHash.
func (mr *MockRunContextMockRecorder) GetBlockHash(number any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetBlockHash", reflect.TypeOf((*MockRunContext)(nil).GetBlockHash), number)
}

// GetCode mocks base method.
func (m *MockRunContext) GetCode(arg0 Address) Code {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetCode", arg0)
	ret0, _ := ret[0].(Code)
	return ret0
}

// GetCode indicates an expected call of GetCode.
func (mr *MockRunContextMockRecorder) GetCode(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetCode", reflect.TypeOf((*MockRunContext)(nil).GetCode), arg0)
}

// GetCodeHash mocks base method.
func (m *MockRunContext) GetCodeHash(arg0 Address) Hash {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetCodeHash", arg0)
	ret0, _ := ret[0].(Hash)
	return ret0
}

// GetCodeHash indicates an expected call of GetCodeHash.
func (mr *MockRunContextMockRecorder) GetCodeHash(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetCodeHash", reflect.TypeOf((*MockRunContext)(nil).GetCodeHash), arg0)
}

// GetCodeSize mocks base method.
func (m *MockRunContext) GetCodeSize(arg0 Address) int {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetCodeSize", arg0)
	ret0, _ := ret[0].(int)
	return ret0
}

// GetCodeSize indicates an expected call of GetCodeSize.
func (mr *MockRunContextMockRecorder) GetCodeSize(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetCodeSize", reflect.TypeOf((*MockRunContext)(nil).GetCodeSize), arg0)
}

// GetCommittedStorage mocks base method.
func (m *MockRunContext) GetCommittedStorage(addr Address, key Key) Word {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetCommittedStorage", addr, key)
	ret0, _ := ret[0].(Word)
	return ret0
}

// GetCommittedStorage indicates an expected call of GetCommittedStorage.
func (mr *MockRunContextMockRecorder) GetCommittedStorage(addr, key any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetCommittedStorage", reflect.TypeOf((*MockRunContext)(nil).GetCommittedStorage), addr, key)
}

// GetLogs mocks base method.
func (m *MockRunContext) GetLogs() []Log {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetLogs")
	ret0, _ := ret[0].([]Log)
	return ret0
}

// GetLogs indicates an expected call of GetLogs.
func (mr *MockRunContextMockRecorder) GetLogs() *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetLogs", reflect.TypeOf((*MockRunContext)(nil).GetLogs))
}

// GetNonce mocks base method.
func (m *MockRunContext) GetNonce(arg0 Address) uint64 {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetNonce", arg0)
	ret0, _ := ret[0].(uint64)
	return ret0
}

// GetNonce indicates an expected call of GetNonce.
func (mr *MockRunContextMockRecorder) GetNonce(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetNonce", reflect.TypeOf((*MockRunContext)(nil).GetNonce), arg0)
}

// GetStorage mocks base method.
func (m *MockRunContext) GetStorage(arg0 Address, arg1 Key) Word {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetStorage", arg0, arg1)
	ret0, _ := ret[0].(Word)
	return ret0
}

// GetStorage indicates an expected call of GetStorage.
func (mr *MockRunContextMockRecorder) GetStorage(arg0, arg1 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetStorage", reflect.TypeOf((*MockRunContext)(nil).GetStorage), arg0, arg1)
}

// GetTransientStorage mocks base method.
func (m *MockRunContext) GetTransientStorage(arg0 Address, arg1 Key) Word {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetTransientStorage", arg0, arg1)
	ret0, _ := ret[0].(Word)
	return ret0
}

// GetTransientStorage indicates an expected call of GetTransientStorage.
func (mr *MockRunContextMockRecorder) GetTransientStorage(arg0, arg1 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetTransientStorage", reflect.TypeOf((*MockRunContext)(nil).GetTransientStorage), arg0, arg1)
}

// HasEmptyStateRoot mocks base method.
func (m *MockRunContext) HasEmptyStateRoot(arg0 Address) bool {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "HasEmptyStateRoot", arg0)
	ret0, _ := ret[0].(bool)
	return ret0
}

// HasEmptyStateRoot indicates an expected call of HasEmptyStateRoot.
func (mr *MockRunContextMockRecorder) HasEmptyStateRoot(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "HasEmptyStateRoot", reflect.TypeOf((*MockRunContext)(nil).HasEmptyStateRoot), arg0)
}

// HasSelfDestructed mocks base method.
func (m *MockRunContext) HasSelfDestructed(addr Address) bool {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "HasSelfDestructed", addr)
	ret0, _ := ret[0].(bool)
	return ret0
}

// HasSelfDestructed indicates an expected call of HasSelfDestructed.
func (mr *MockRunContextMockRecorder) HasSelfDestructed(addr any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "HasSelfDestructed", reflect.TypeOf((*MockRunContext)(nil).HasSelfDestructed), addr)
}

// IsAddressInAccessList mocks base method.
func (m *MockRunContext) IsAddressInAccessList(addr Address) bool {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "IsAddressInAccessList", addr)
	ret0, _ := ret[0].(bool)
	return ret0
}

// IsAddressInAccessList indicates an expected call of IsAddressInAccessList.
func (mr *MockRunContextMockRecorder) IsAddressInAccessList(addr any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "IsAddressInAccessList", reflect.TypeOf((*MockRunContext)(nil).IsAddressInAccessList), addr)
}

// IsSlotInAccessList mocks base method.
func (m *MockRunContext) IsSlotInAccessList(addr Address, key Key) (bool, bool) {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "IsSlotInAccessList", addr, key)
	ret0, _ := ret[0].(bool)
	ret1, _ := ret[1].(bool)
	return ret0, ret1
}

// IsSlotInAccessList indicates an expected call of IsSlotInAccessList.
func (mr *MockRunContextMockRecorder) IsSlotInAccessList(addr, key any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "IsSlotInAccessList", reflect.TypeOf((*MockRunContext)(nil).IsSlotInAccessList), addr, key)
}

// RestoreSnapshot mocks base method.
func (m *MockRunContext) RestoreSnapshot(arg0 Snapshot) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "RestoreSnapshot", arg0)
}

// RestoreSnapshot indicates an expected call of RestoreSnapshot.
func (mr *MockRunContextMockRecorder) RestoreSnapshot(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "RestoreSnapshot", reflect.TypeOf((*MockRunContext)(nil).RestoreSnapshot), arg0)
}

// SelfDestruct mocks base method.
func (m *MockRunContext) SelfDestruct(addr, beneficiary Address) bool {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "SelfDestruct", addr, beneficiary)
	ret0, _ := ret[0].(bool)
	return ret0
}

// SelfDestruct indicates an expected call of SelfDestruct.
func (mr *MockRunContextMockRecorder) SelfDestruct(addr, beneficiary any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "SelfDestruct", reflect.TypeOf((*MockRunContext)(nil).SelfDestruct), addr, beneficiary)
}

// SetBalance mocks base method.
func (m *MockRunContext) SetBalance(arg0 Address, arg1 Value) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "SetBalance", arg0, arg1)
}

// SetBalance indicates an expected call of SetBalance.
func (mr *MockRunContextMockRecorder) SetBalance(arg0, arg1 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "SetBalance", reflect.TypeOf((*MockRunContext)(nil).SetBalance), arg0, arg1)
}

// SetCode mocks base method.
func (m *MockRunContext) SetCode(arg0 Address, arg1 Code) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "SetCode", arg0, arg1)
}

// SetCode indicates an expected call of SetCode.
func (mr *MockRunContextMockRecorder) SetCode(arg0, arg1 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "SetCode", reflect.TypeOf((*MockRunContext)(nil).SetCode), arg0, arg1)
}

// SetNonce mocks base method.
func (m *MockRunContext) SetNonce(arg0 Address, arg1 uint64) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "SetNonce", arg0, arg1)
}

// SetNonce indicates an expected call of SetNonce.
func (mr *MockRunContextMockRecorder) SetNonce(arg0, arg1 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "SetNonce", reflect.TypeOf((*MockRunContext)(nil).SetNonce), arg0, arg1)
}

// SetStorage mocks base method.
func (m *MockRunContext) SetStorage(arg0 Address, arg1 Key, arg2 Word) StorageStatus {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "SetStorage", arg0, arg1, arg2)
	ret0, _ := ret[0].(StorageStatus)
	return ret0
}

// SetStorage indicates an expected call of SetStorage.
func (mr *MockRunContextMockRecorder) SetStorage(arg0, arg1, arg2 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "SetStorage", reflect.TypeOf((*MockRunContext)(nil).SetStorage), arg0, arg1, arg2)
}

// SetTransientStorage mocks base method.
func (m *MockRunContext) SetTransientStorage(arg0 Address, arg1 Key, arg2 Word) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "SetTransientStorage", arg0, arg1, arg2)
}

// SetTransientStorage indicates an expected call of SetTransientStorage.
func (mr *MockRunContextMockRecorder) SetTransientStorage(arg0, arg1, arg2 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "SetTransientStorage", reflect.TypeOf((*MockRunContext)(nil).SetTransientStorage), arg0, arg1, arg2)
}

// MockTransactionContext is a mock of TransactionContext interface.
type MockTransactionContext struct {
	ctrl     *gomock.Controller
	recorder *MockTransactionContextMockRecorder
	isgomock struct{}
}

// MockTransactionContextMockRecorder is the mock recorder for MockTransactionContext.
type MockTransactionContextMockRecorder struct {
	mock *MockTransactionContext
}

// NewMockTransactionContext creates a new mock instance.
func NewMockTransactionContext(ctrl *gomock.Controller) *MockTransactionContext {
	mock := &MockTransactionContext{ctrl: ctrl}
	mock.recorder = &MockTransactionContextMockRecorder{mock}
	return mock
}

// EXPECT returns an object that allows the caller to indicate expected use.
func (m *MockTransactionContext) EXPECT() *MockTransactionContextMockRecorder {
	return m.recorder
}

// AccessAccount mocks base method.
func (m *MockTransactionContext) AccessAccount(arg0 Address) AccessStatus {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "AccessAccount", arg0)
	ret0, _ := ret[0].(AccessStatus)
	return ret0
}

// AccessAccount indicates an expected call of AccessAccount.
func (mr *MockTransactionContextMockRecorder) AccessAccount(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "AccessAccount", reflect.TypeOf((*MockTransactionContext)(nil).AccessAccount), arg0)
}

// AccessStorage mocks base method.
func (m *MockTransactionContext) AccessStorage(arg0 Address, arg1 Key) AccessStatus {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "AccessStorage", arg0, arg1)
	ret0, _ := ret[0].(AccessStatus)
	return ret0
}

// AccessStorage indicates an expected call of AccessStorage.
func (mr *MockTransactionContextMockRecorder) AccessStorage(arg0, arg1 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "AccessStorage", reflect.TypeOf((*MockTransactionContext)(nil).AccessStorage), arg0, arg1)
}

// AccountExists mocks base method.
func (m *MockTransactionContext) AccountExists(arg0 Address) bool {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "AccountExists", arg0)
	ret0, _ := ret[0].(bool)
	return ret0
}

// AccountExists indicates an expected call of AccountExists.
func (mr *MockTransactionContextMockRecorder) AccountExists(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "AccountExists", reflect.TypeOf((*MockTransactionContext)(nil).AccountExists), arg0)
}

// CreateAccount mocks base method.
func (m *MockTransactionContext) CreateAccount(arg0 Address) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "CreateAccount", arg0)
}

// CreateAccount indicates an expected call of CreateAccount.
func (mr *MockTransactionContextMockRecorder) CreateAccount(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "CreateAccount", reflect.TypeOf((*MockTransactionContext)(nil).CreateAccount), arg0)
}

// CreateSnapshot mocks base method.
func (m *MockTransactionContext) CreateSnapshot() Snapshot {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "CreateSnapshot")
	ret0, _ := ret[0].(Snapshot)
	return ret0
}

// CreateSnapshot indicates an expected call of CreateSnapshot.
func (mr *MockTransactionContextMockRecorder) CreateSnapshot() *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "CreateSnapshot", reflect.TypeOf((*MockTransactionContext)(nil).CreateSnapshot))
}

// EmitLog mocks base method.
func (m *MockTransactionContext) EmitLog(arg0 Log) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "EmitLog", arg0)
}

// EmitLog indicates an expected call of EmitLog.
func (mr *MockTransactionContextMockRecorder) EmitLog(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "EmitLog", reflect.TypeOf((*MockTransactionContext)(nil).EmitLog), arg0)
}

// GetBalance mocks base method.
func (m *MockTransactionContext) GetBalance(arg0 Address) Value {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetBalance", arg0)
	ret0, _ := ret[0].(Value)
	return ret0
}

// GetBalance indicates an expected call of GetBalance.
func (mr *MockTransactionContextMockRecorder) GetBalance(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetBalance", reflect.TypeOf((*MockTransactionContext)(nil).GetBalance), arg0)
}

// GetBlockHash mocks base method.
func (m *MockTransactionContext) GetBlockHash(number int64) Hash {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetBlockHash", number)
	ret0, _ := ret[0].(Hash)
	return ret0
}

// GetBlockHash indicates an expected call of GetBlockHash.
func (mr *MockTransactionContextMockRecorder) GetBlockHash(number any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetBlockHash", reflect.TypeOf((*MockTransactionContext)(nil).GetBlockHash), number)
}

// GetCode mocks base method.
func (m *MockTransactionContext) GetCode(arg0 Address) Code {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetCode", arg0)
	ret0, _ := ret[0].(Code)
	return ret0
}

// GetCode indicates an expected call of GetCode.
func (mr *MockTransactionContextMockRecorder) GetCode(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetCode", reflect.TypeOf((*MockTransactionContext)(nil).GetCode), arg0)
}

// GetCodeHash mocks base method.
func (m *MockTransactionContext) GetCodeHash(arg0 Address) Hash {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetCodeHash", arg0)
	ret0, _ := ret[0].(Hash)
	return ret0
}

// GetCodeHash indicates an expected call of GetCodeHash.
func (mr *MockTransactionContextMockRecorder) GetCodeHash(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetCodeHash", reflect.TypeOf((*MockTransactionContext)(nil).GetCodeHash), arg0)
}

// GetCodeSize mocks base method.
func (m *MockTransactionContext) GetCodeSize(arg0 Address) int {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetCodeSize", arg0)
	ret0, _ := ret[0].(int)
	return ret0
}

// GetCodeSize indicates an expected call of GetCodeSize.
func (mr *MockTransactionContextMockRecorder) GetCodeSize(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetCodeSize", reflect.TypeOf((*MockTransactionContext)(nil).GetCodeSize), arg0)
}

// GetCommittedStorage mocks base method.
func (m *MockTransactionContext) GetCommittedStorage(addr Address, key Key) Word {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetCommittedStorage", addr, key)
	ret0, _ := ret[0].(Word)
	return ret0
}

// GetCommittedStorage indicates an expected call of GetCommittedStorage.
func (mr *MockTransactionContextMockRecorder) GetCommittedStorage(addr, key any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetCommittedStorage", reflect.TypeOf((*MockTransactionContext)(nil).GetCommittedStorage), addr, key)
}

// GetLogs mocks base method.
func (m *MockTransactionContext) GetLogs() []Log {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetLogs")
	ret0, _ := ret[0].([]Log)
	return ret0
}

// GetLogs indicates an expected call of GetLogs.
func (mr *MockTransactionContextMockRecorder) GetLogs() *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetLogs", reflect.TypeOf((*MockTransactionContext)(nil).GetLogs))
}

// GetNonce mocks base method.
func (m *MockTransactionContext) GetNonce(arg0 Address) uint64 {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetNonce", arg0)
	ret0, _ := ret[0].(uint64)
	return ret0
}

// GetNonce indicates an expected call of GetNonce.
func (mr *MockTransactionContextMockRecorder) GetNonce(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetNonce", reflect.TypeOf((*MockTransactionContext)(nil).GetNonce), arg0)
}

// GetStorage mocks base method.
func (m *MockTransactionContext) GetStorage(arg0 Address, arg1 Key) Word {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetStorage", arg0, arg1)
	ret0, _ := ret[0].(Word)
	return ret0
}

// GetStorage indicates an expected call of GetStorage.
func (mr *MockTransactionContextMockRecorder) GetStorage(arg0, arg1 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetStorage", reflect.TypeOf((*MockTransactionContext)(nil).GetStorage), arg0, arg1)
}

// GetTransientStorage mocks base method.
func (m *MockTransactionContext) GetTransientStorage(arg0 Address, arg1 Key) Word {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "GetTransientStorage", arg0, arg1)
	ret0, _ := ret[0].(Word)
	return ret0
}

// GetTransientStorage indicates an expected call of GetTransientStorage.
func (mr *MockTransactionContextMockRecorder) GetTransientStorage(arg0, arg1 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "GetTransientStorage", reflect.TypeOf((*MockTransactionContext)(nil).GetTransientStorage), arg0, arg1)
}

// HasEmptyStateRoot mocks base method.
func (m *MockTransactionContext) HasEmptyStateRoot(arg0 Address) bool {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "HasEmptyStateRoot", arg0)
	ret0, _ := ret[0].(bool)
	return ret0
}

// HasEmptyStateRoot indicates an expected call of HasEmptyStateRoot.
func (mr *MockTransactionContextMockRecorder) HasEmptyStateRoot(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "HasEmptyStateRoot", reflect.TypeOf((*MockTransactionContext)(nil).HasEmptyStateRoot), arg0)
}

// HasSelfDestructed mocks base method.
func (m *MockTransactionContext) HasSelfDestructed(addr Address) bool {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "HasSelfDestructed", addr)
	ret0, _ := ret[0].(bool)
	return ret0
}

// HasSelfDestructed indicates an expected call of HasSelfDestructed.
func (mr *MockTransactionContextMockRecorder) HasSelfDestructed(addr any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "HasSelfDestructed", reflect.TypeOf((*MockTransactionContext)(nil).HasSelfDestructed), addr)
}

// IsAddressInAccessList mocks base method.
func (m *MockTransactionContext) IsAddressInAccessList(addr Address) bool {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "IsAddressInAccessList", addr)
	ret0, _ := ret[0].(bool)
	return ret0
}

// IsAddressInAccessList indicates an expected call of IsAddressInAccessList.
func (mr *MockTransactionContextMockRecorder) IsAddressInAccessList(addr any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "IsAddressInAccessList", reflect.TypeOf((*MockTransactionContext)(nil).IsAddressInAccessList), addr)
}

// IsSlotInAccessList mocks base method.
func (m *MockTransactionContext) IsSlotInAccessList(addr Address, key Key) (bool, bool) {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "IsSlotInAccessList", addr, key)
	ret0, _ := ret[0].(bool)
	ret1, _ := ret[1].(bool)
	return ret0, ret1
}

// IsSlotInAccessList indicates an expected call of IsSlotInAccessList.
func (mr *MockTransactionContextMockRecorder) IsSlotInAccessList(addr, key any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "IsSlotInAccessList", reflect.TypeOf((*MockTransactionContext)(nil).IsSlotInAccessList), addr, key)
}

// RestoreSnapshot mocks base method.
func (m *MockTransactionContext) RestoreSnapshot(arg0 Snapshot) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "RestoreSnapshot", arg0)
}

// RestoreSnapshot indicates an expected call of RestoreSnapshot.
func (mr *MockTransactionContextMockRecorder) RestoreSnapshot(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "RestoreSnapshot", reflect.TypeOf((*MockTransactionContext)(nil).RestoreSnapshot), arg0)
}

// SelfDestruct mocks base method.
func (m *MockTransactionContext) SelfDestruct(addr, beneficiary Address) bool {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "SelfDestruct", addr, beneficiary)
	ret0, _ := ret[0].(bool)
	return ret0
}

// SelfDestruct indicates an expected call of SelfDestruct.
func (mr *MockTransactionContextMockRecorder) SelfDestruct(addr, beneficiary any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "SelfDestruct", reflect.TypeOf((*MockTransactionContext)(nil).SelfDestruct), addr, beneficiary)
}

// SetBalance mocks base method.
func (m *MockTransactionContext) SetBalance(arg0 Address, arg1 Value) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "SetBalance", arg0, arg1)
}

// SetBalance indicates an expected call of SetBalance.
func (mr *MockTransactionContextMockRecorder) SetBalance(arg0, arg1 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "SetBalance", reflect.TypeOf((*MockTransactionContext)(nil).SetBalance), arg0, arg1)
}

// SetCode mocks base method.
func (m *MockTransactionContext) SetCode(arg0 Address, arg1 Code) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "SetCode", arg0, arg1)
}

// SetCode indicates an expected call of SetCode.
func (mr *MockTransactionContextMockRecorder) SetCode(arg0, arg1 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "SetCode", reflect.TypeOf((*MockTransactionContext)(nil).SetCode), arg0, arg1)
}

// SetNonce mocks base method.
func (m *MockTransactionContext) SetNonce(arg0 Address, arg1 uint64) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "SetNonce", arg0, arg1)
}

// SetNonce indicates an expected call of SetNonce.
func (mr *MockTransactionContextMockRecorder) SetNonce(arg0, arg1 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "SetNonce", reflect.TypeOf((*MockTransactionContext)(nil).SetNonce), arg0, arg1)
}

// SetStorage mocks base method.
func (m *MockTransactionContext) SetStorage(arg0 Address, arg1 Key, arg2 Word) StorageStatus {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "SetStorage", arg0, arg1, arg2)
	ret0, _ := ret[0].(StorageStatus)
	return ret0
}

// SetStorage indicates an expected call of SetStorage.
func (mr *MockTransactionContextMockRecorder) SetStorage(arg0, arg1, arg2 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "SetStorage", reflect.TypeOf((*MockTransactionContext)(nil).SetStorage), arg0, arg1, arg2)
}

// SetTransientStorage mocks base method.
func (m *MockTransactionContext) SetTransientStorage(arg0 Address, arg1 Key, arg2 Word) {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "SetTransientStorage", arg0, arg1, arg2)
}

// SetTransientStorage indicates an expected call of SetTransientStorage.
func (mr *MockTransactionContextMockRecorder) SetTransientStorage(arg0, arg1, arg2 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "SetTransientStorage", reflect.TypeOf((*MockTransactionContext)(nil).SetTransientStorage), arg0, arg1, arg2)
}

// MockProfilingInterpreter is a mock of ProfilingInterpreter interface.
type MockProfilingInterpreter struct {
	ctrl     *gomock.Controller
	recorder *MockProfilingInterpreterMockRecorder
	isgomock struct{}
}

// MockProfilingInterpreterMockRecorder is the mock recorder for MockProfilingInterpreter.
type MockProfilingInterpreterMockRecorder struct {
	mock *MockProfilingInterpreter
}

// NewMockProfilingInterpreter creates a new mock instance.
func NewMockProfilingInterpreter(ctrl *gomock.Controller) *MockProfilingInterpreter {
	mock := &MockProfilingInterpreter{ctrl: ctrl}
	mock.recorder = &MockProfilingInterpreterMockRecorder{mock}
	return mock
}

// EXPECT returns an object that allows the caller to indicate expected use.
func (m *MockProfilingInterpreter) EXPECT() *MockProfilingInterpreterMockRecorder {
	return m.recorder
}

// DumpProfile mocks base method.
func (m *MockProfilingInterpreter) DumpProfile() {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "DumpProfile")
}

// DumpProfile indicates an expected call of DumpProfile.
func (mr *MockProfilingInterpreterMockRecorder) DumpProfile() *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "DumpProfile", reflect.TypeOf((*MockProfilingInterpreter)(nil).DumpProfile))
}

// ResetProfile mocks base method.
func (m *MockProfilingInterpreter) ResetProfile() {
	m.ctrl.T.Helper()
	m.ctrl.Call(m, "ResetProfile")
}

// ResetProfile indicates an expected call of ResetProfile.
func (mr *MockProfilingInterpreterMockRecorder) ResetProfile() *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "ResetProfile", reflect.TypeOf((*MockProfilingInterpreter)(nil).ResetProfile))
}

// Run mocks base method.
func (m *MockProfilingInterpreter) Run(arg0 Parameters) (Result, error) {
	m.ctrl.T.Helper()
	ret := m.ctrl.Call(m, "Run", arg0)
	ret0, _ := ret[0].(Result)
	ret1, _ := ret[1].(error)
	return ret0, ret1
}

// Run indicates an expected call of Run.
func (mr *MockProfilingInterpreterMockRecorder) Run(arg0 any) *gomock.Call {
	mr.mock.ctrl.T.Helper()
	return mr.mock.ctrl.RecordCallWithMethodType(mr.mock, "Run", reflect.TypeOf((*MockProfilingInterpreter)(nil).Run), arg0)
}
