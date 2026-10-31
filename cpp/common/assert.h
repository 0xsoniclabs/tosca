// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

#pragma once

#include <cstdio>

#include "common/macros.h"

#if TOSCA_ASSERT_ENABLED
#define TOSCA_ASSERT(condition)                                                                \
  do {                                                                                         \
    if (!(condition)) [[unlikely]] {                                                           \
      ::std::fprintf(stderr, "%s:%d: Assertion failed: %s\n", __FILE__, __LINE__, #condition); \
      TOSCA_DEBUG_BREAK();                                                                     \
    }                                                                                          \
  } while (0)
#else
#define TOSCA_ASSERT(condition)
#endif
