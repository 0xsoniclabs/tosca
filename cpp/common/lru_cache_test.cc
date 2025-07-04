// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

#include "common/lru_cache.h"

#include <gtest/gtest.h>

namespace tosca::evmzero {
namespace {

TEST(LruCacheTest, Init) { LruCache<int, int, 32> cache; }

TEST(LruCacheTest, GetMissing) {
  LruCache<int, int, 32> cache;
  EXPECT_EQ(cache.Get(0), std::nullopt);
}

TEST(LruCacheTest, Insert) {
  LruCache<int, int, 32> cache;

  auto element = cache.InsertOrAssign(0, 42);
  EXPECT_EQ(element, 42);
  EXPECT_EQ(*cache.Get(0), 42);
}

TEST(LruCacheTest, Assign) {
  LruCache<int, int, 32> cache;

  cache.InsertOrAssign(0, 42);
  auto element = cache.InsertOrAssign(0, 23);
  EXPECT_EQ(element, 23);
  EXPECT_EQ(*cache.Get(0), 23);
}

TEST(LruCacheTest, GetOrInsert) {
  LruCache<int, int, 32> cache;

  EXPECT_EQ(42, cache.GetOrInsert(0, []() { return 42; }));

  EXPECT_EQ(42, cache.GetOrInsert(0, []() {
    EXPECT_TRUE(false);  // Should not be executed!
    return 0;
  }));

  EXPECT_EQ(21, cache.GetOrInsert(1, []() { return 21; }));
}

TEST(LruCacheTest, LeastRecentlyUsedRemoved) {
  {
    LruCache<int, int, 2> cache;
    cache.InsertOrAssign(0, 40);
    cache.InsertOrAssign(1, 41);

    cache.Get(0);
    cache.InsertOrAssign(2, 42);  // removes key 1
    EXPECT_EQ(*cache.Get(0), 40);
    EXPECT_EQ(*cache.Get(2), 42);
    EXPECT_EQ(cache.Get(1), std::nullopt);
  }

  {
    LruCache<int, int, 2> cache;
    cache.InsertOrAssign(0, 40);
    cache.InsertOrAssign(1, 41);

    cache.Get(1);
    cache.InsertOrAssign(2, 42);  // removes key 0
    EXPECT_EQ(*cache.Get(1), 41);
    EXPECT_EQ(*cache.Get(2), 42);
    EXPECT_EQ(cache.Get(0), std::nullopt);
  }
}

TEST(LruCacheTest, Clear) {
  LruCache<int, int, 32> cache;
  cache.InsertOrAssign(0, 42);
  cache.Clear();
}

}  // namespace
}  // namespace tosca::evmzero
